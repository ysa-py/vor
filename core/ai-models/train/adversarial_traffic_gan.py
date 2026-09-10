#!/usr/bin/env python3
"""
MICAFP-UnifiedShield-6.0 — Adversarial Traffic GAN Training Script

Trains a Wasserstein GAN with Gradient Penalty (WGAN-GP) that generates
traffic patterns indistinguishable from legitimate HTTPS browsing/video
streaming/downloads, as measured by a Discriminator trained on FAVA DPI
signatures.

Output:
  - ONNX model for Rust inference (INT8 quantized)
  - Generator + Discriminator checkpoints
  - Training metrics (TensorBoard)

Usage:
  python adversarial_traffic_gan.py \
      --data-dir ./traffic_captures \
      --epochs 500 \
      --batch-size 64 \
      --export-onnx ./models/traffic_gan.onnx

Training data should be .npy files containing captured traffic features:
  - packet_sizes:     shape (N, max_packets)    — packet sizes in bytes
  - inter_arrival_times: shape (N, max_packets) — inter-arrival µs
  - byte_histograms:  shape (N, 256)            — byte frequency histogram
"""

from __future__ import annotations

import argparse
import os
import pathlib
import time
from typing import Tuple

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import DataLoader, Dataset, TensorDataset
from torch.utils.tensorboard import SummaryWriter

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

MAX_PACKETS = 64          # Maximum packets per flow
BYTE_HIST_DIM = 256       # Byte histogram bins
NOISE_DIM = 128           # Generator noise vector dimension
TRAFFIC_TYPE_DIM = 3      # One-hot: [browsing, video, download]
FEATURE_DIM = MAX_PACKETS * 2 + BYTE_HIST_DIM  # Total feature dimension

# ---------------------------------------------------------------------------
# Discriminator (1D-CNN)
# ---------------------------------------------------------------------------


class Discriminator(nn.Module):
    """
    1D-CNN Discriminator trained on FAVA DPI signatures.

    Input: concatenated tensor of
      - packet_sizes (max_packets)
      - inter_arrival_times (max_packets)
      - byte_histograms (256)

    Architecture:
      Conv1d → BatchNorm → LeakyReLU → Dropout → Conv1d → Sigmoid
    """

    def __init__(self, feature_dim: int = FEATURE_DIM, dropout: float = 0.3):
        super().__init__()

        self.feature_dim = feature_dim

        # Three parallel branches for different feature types
        # Branch 1: Packet sizes
        self.conv_pkt = nn.Sequential(
            nn.Conv1d(1, 64, kernel_size=5, stride=2, padding=2),
            nn.BatchNorm1d(64),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
            nn.Conv1d(64, 128, kernel_size=3, stride=2, padding=1),
            nn.BatchNorm1d(128),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
        )

        # Branch 2: Inter-arrival times
        self.conv_iat = nn.Sequential(
            nn.Conv1d(1, 64, kernel_size=5, stride=2, padding=2),
            nn.BatchNorm1d(64),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
            nn.Conv1d(64, 128, kernel_size=3, stride=2, padding=1),
            nn.BatchNorm1d(128),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
        )

        # Branch 3: Byte histogram
        self.conv_hist = nn.Sequential(
            nn.Conv1d(1, 64, kernel_size=7, stride=2, padding=3),
            nn.BatchNorm1d(64),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
            nn.Conv1d(64, 128, kernel_size=3, stride=2, padding=1),
            nn.BatchNorm1d(128),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
        )

        # Calculate flattened sizes after convolutions
        self._calc_sizes()

        # Combined fully-connected layers
        combined_dim = self.pkt_flat + self.iat_flat + self.hist_flat

        self.fc = nn.Sequential(
            nn.Linear(combined_dim, 512),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
            nn.Linear(512, 256),
            nn.LeakyReLU(0.2, inplace=True),
            nn.Dropout(dropout),
            nn.Linear(256, 1),  # No sigmoid for WGAN — output is unbounded
        )

    def _calc_sizes(self):
        """Calculate output sizes after conv layers for each branch."""
        with torch.no_grad():
            # Packet sizes branch
            dummy = torch.zeros(1, 1, MAX_PACKETS)
            out = self.conv_pkt(dummy)
            self.pkt_flat = out.view(1, -1).shape[1]

            # Inter-arrival times branch
            dummy = torch.zeros(1, 1, MAX_PACKETS)
            out = self.conv_iat(dummy)
            self.iat_flat = out.view(1, -1).shape[1]

            # Byte histogram branch
            dummy = torch.zeros(1, 1, BYTE_HIST_DIM)
            out = self.conv_hist(dummy)
            self.hist_flat = out.view(1, -1).shape[1]

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """
        Args:
            x: (batch, feature_dim) — concatenated features
        Returns:
            (batch, 1) — critic score (unbounded for WGAN)
        """
        batch_size = x.shape[0]

        # Split input into three feature groups
        pkt_sizes = x[:, :MAX_PACKETS].unsqueeze(1)           # (B, 1, MAX_PACKETS)
        iat = x[:, MAX_PACKETS:2*MAX_PACKETS].unsqueeze(1)    # (B, 1, MAX_PACKETS)
        hist = x[:, 2*MAX_PACKETS:].unsqueeze(1)               # (B, 1, 256)

        # Process each branch
        pkt_feat = self.conv_pkt(pkt_sizes).view(batch_size, -1)
        iat_feat = self.conv_iat(iat).view(batch_size, -1)
        hist_feat = self.conv_hist(hist).view(batch_size, -1)

        # Concatenate and classify
        combined = torch.cat([pkt_feat, iat_feat, hist_feat], dim=1)
        return self.fc(combined)


# ---------------------------------------------------------------------------
# Generator
# ---------------------------------------------------------------------------


class Generator(nn.Module):
    """
    Generates traffic patterns that fool the Discriminator.

    Input: noise vector + target_traffic_type (one-hot)
    Output: (packet_sizes, inter_arrival_times, byte_histograms)
    """

    def __init__(self, noise_dim: int = NOISE_DIM, traffic_type_dim: int = TRAFFIC_TYPE_DIM):
        super().__init__()

        input_dim = noise_dim + traffic_type_dim

        # Shared backbone
        self.backbone = nn.Sequential(
            nn.Linear(input_dim, 512),
            nn.BatchNorm1d(512),
            nn.ReLU(inplace=True),
            nn.Linear(512, 1024),
            nn.BatchNorm1d(1024),
            nn.ReLU(inplace=True),
            nn.Linear(1024, 2048),
            nn.BatchNorm1d(2048),
            nn.ReLU(inplace=True),
        )

        # Packet sizes head (positive integers, up to MTU 1500)
        self.pkt_head = nn.Sequential(
            nn.Linear(2048, 512),
            nn.BatchNorm1d(512),
            nn.ReLU(inplace=True),
            nn.Linear(512, MAX_PACKETS),
            nn.Sigmoid(),  # [0, 1] → scale to [0, 1500] later
        )

        # Inter-arrival times head (positive, log-scale)
        self.iat_head = nn.Sequential(
            nn.Linear(2048, 512),
            nn.BatchNorm1d(512),
            nn.ReLU(inplace=True),
            nn.Linear(512, MAX_PACKETS),
            nn.Softplus(),  # Always positive
        )

        # Byte histogram head (must sum to ~1, i.e., a distribution)
        self.hist_head = nn.Sequential(
            nn.Linear(2048, 512),
            nn.BatchNorm1d(512),
            nn.ReLU(inplace=True),
            nn.Linear(512, BYTE_HIST_DIM),
            nn.Softmax(dim=1),  # Normalized distribution
        )

    def forward(
        self, noise: torch.Tensor, traffic_type: torch.Tensor
    ) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """
        Args:
            noise: (batch, noise_dim) — random noise
            traffic_type: (batch, traffic_type_dim) — one-hot encoding
        Returns:
            packet_sizes: (batch, MAX_PACKETS) — in [0, 1500]
            inter_arrival_times: (batch, MAX_PACKETS) — in microseconds
            byte_histograms: (batch, 256) — normalized distribution
        """
        x = torch.cat([noise, traffic_type], dim=1)
        features = self.backbone(x)

        packet_sizes = self.pkt_head(features) * 1500.0        # Scale to bytes
        inter_arrival_times = self.iat_head(features) * 1000.0  # Scale to µs
        byte_histograms = self.hist_head(features)

        return packet_sizes, inter_arrival_times, byte_histograms

    def generate_feature_vector(
        self, noise: torch.Tensor, traffic_type: torch.Tensor
    ) -> torch.Tensor:
        """Generate the full concatenated feature vector."""
        pkt, iat, hist = self.forward(noise, traffic_type)
        return torch.cat([pkt, iat, hist], dim=1)


# ---------------------------------------------------------------------------
# Gradient Penalty (WGAN-GP)
# ---------------------------------------------------------------------------


def compute_gradient_penalty(
    discriminator: Discriminator,
    real_samples: torch.Tensor,
    fake_samples: torch.Tensor,
    device: torch.device,
) -> torch.Tensor:
    """
    Compute the gradient penalty for WGAN-GP.

    Interpolates between real and fake samples and penalizes the
    gradient norm deviating from 1.
    """
    batch_size = real_samples.shape[0]
    alpha = torch.rand(batch_size, 1, device=device)
    alpha = alpha.expand_as(real_samples)

    interpolated = (alpha * real_samples + (1 - alpha) * fake_samples).requires_grad_(True)

    d_interpolated = discriminator(interpolated)

    gradients = torch.autograd.grad(
        outputs=d_interpolated,
        inputs=interpolated,
        grad_outputs=torch.ones_like(d_interpolated),
        create_graph=True,
        retain_graph=True,
    )[0]

    gradients = gradients.view(batch_size, -1)
    gradient_penalty = ((gradients.norm(2, dim=1) - 1) ** 2).mean()
    return gradient_penalty


# ---------------------------------------------------------------------------
# Traffic Dataset
# ---------------------------------------------------------------------------


class TrafficDataset(Dataset):
    """
    Loads traffic capture data from .npy files.

    Expected files in data_dir:
      - packet_sizes.npy         (N, max_packets)
      - inter_arrival_times.npy  (N, max_packets)
      - byte_histograms.npy      (N, 256)
      - traffic_types.npy        (N,) — 0=browsing, 1=video, 2=download
    """

    def __init__(self, data_dir: str, max_packets: int = MAX_PACKETS):
        data_path = pathlib.Path(data_dir)

        pkt_sizes = np.load(data_path / "packet_sizes.npy").astype(np.float32)
        iat = np.load(data_path / "inter_arrival_times.npy").astype(np.float32)
        hist = np.load(data_path / "byte_histograms.npy").astype(np.float32)
        types = np.load(data_path / "traffic_types.npy").astype(np.int64)

        # Pad/truncate to max_packets
        pkt_sizes = self._pad_truncate(pkt_sizes, max_packets)
        iat = self._pad_truncate(iat, max_packets)

        # Normalize packet sizes to [0, 1]
        pkt_sizes = pkt_sizes / 1500.0

        # Normalize inter-arrival times (log-scale)
        iat = np.log1p(iat) / 15.0  # Assume max ~3.3M µs (3.3s)

        # Normalize byte histograms (ensure they sum to 1)
        row_sums = hist.sum(axis=1, keepdims=True)
        row_sums[row_sums == 0] = 1.0
        hist = hist / row_sums

        self.pkt_sizes = torch.from_numpy(pkt_sizes)
        self.iat = torch.from_numpy(iat)
        self.hist = torch.from_numpy(hist)
        self.types = torch.from_numpy(types)

    @staticmethod
    def _pad_truncate(arr: np.ndarray, max_len: int) -> np.ndarray:
        """Pad with zeros or truncate to max_len along axis 1."""
        N, L = arr.shape
        if L >= max_len:
            return arr[:, :max_len]
        padded = np.zeros((N, max_len), dtype=arr.dtype)
        padded[:, :L] = arr
        return padded

    def __len__(self) -> int:
        return len(self.types)

    def __getitem__(self, idx: int) -> Tuple[torch.Tensor, torch.Tensor]:
        """
        Returns:
            features: (feature_dim,) — concatenated tensor
            traffic_type_onehot: (traffic_type_dim,) — one-hot
        """
        pkt = self.pkt_sizes[idx]
        iat = self.iat[idx]
        hist = self.hist[idx]

        features = torch.cat([pkt, iat, hist])

        # One-hot encode traffic type
        type_onehot = F.one_hot(self.types[idx], num_classes=TRAFFIC_TYPE_DIM).float()

        return features, type_onehot


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------


def train(
    data_dir: str,
    epochs: int = 500,
    batch_size: int = 64,
    lr_d: float = 1e-4,
    lr_g: float = 1e-4,
    n_critic: int = 5,
    lambda_gp: float = 10.0,
    output_dir: str = "./models",
    device_str: str = "auto",
) -> None:
    """Main training loop."""

    # Device selection
    if device_str == "auto":
        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    else:
        device = torch.device(device_str)
    print(f"[Shield-GAN] Training on: {device}")

    # Create output directory
    out_path = pathlib.Path(output_dir)
    out_path.mkdir(parents=True, exist_ok=True)

    # Load data
    print(f"[Shield-GAN] Loading data from: {data_dir}")
    dataset = TrafficDataset(data_dir)
    dataloader = DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=True,
        num_workers=4,
        pin_memory=True,
        drop_last=True,
    )
    print(f"[Shield-GAN] Dataset size: {len(dataset)} samples, {len(dataloader)} batches")

    # Initialize models
    generator = Generator(NOISE_DIM, TRAFFIC_TYPE_DIM).to(device)
    discriminator = Discriminator(FEATURE_DIM).to(device)

    print(f"[Shield-GAN] Generator params: {sum(p.numel() for p in generator.parameters()):,}")
    print(f"[Shield-GAN] Discriminator params: {sum(p.numel() for p in discriminator.parameters()):,}")

    # Optimizers
    opt_g = torch.optim.Adam(generator.parameters(), lr=lr_g, betas=(0.5, 0.999))
    opt_d = torch.optim.Adam(discriminator.parameters(), lr=lr_d, betas=(0.5, 0.999))

    # TensorBoard
    writer = SummaryWriter(log_dir=str(out_path / "logs"))

    # Training loop
    global_step = 0
    best_d_loss = float("inf")

    for epoch in range(1, epochs + 1):
        epoch_start = time.time()
        d_losses = []
        g_losses = []

        for batch_idx, (real_features, traffic_type) in enumerate(dataloader):
            real_features = real_features.to(device)
            traffic_type = traffic_type.to(device)
            batch_size_actual = real_features.shape[0]

            # -----------------------------------------------------------
            # Train Discriminator (critic)
            # -----------------------------------------------------------
            for _ in range(n_critic):
                opt_d.zero_grad()

                # Real samples
                d_real = discriminator(real_features)

                # Fake samples
                noise = torch.randn(batch_size_actual, NOISE_DIM, device=device)
                fake_features = generator.generate_feature_vector(noise, traffic_type).detach()
                d_fake = discriminator(fake_features)

                # Gradient penalty
                gp = compute_gradient_penalty(discriminator, real_features, fake_features, device)

                # WGAN-GP loss
                d_loss = d_fake.mean() - d_real.mean() + lambda_gp * gp
                d_loss.backward()
                opt_d.step()

            d_losses.append(d_loss.item())

            # -----------------------------------------------------------
            # Train Generator
            # -----------------------------------------------------------
            opt_g.zero_grad()

            noise = torch.randn(batch_size_actual, NOISE_DIM, device=device)
            fake_features = generator.generate_feature_vector(noise, traffic_type)
            d_fake = discriminator(fake_features)

            g_loss = -d_fake.mean()
            g_loss.backward()
            opt_g.step()

            g_losses.append(g_loss.item())

            # TensorBoard logging
            if global_step % 100 == 0:
                writer.add_scalar("Loss/Discriminator", d_loss.item(), global_step)
                writer.add_scalar("Loss/Generator", g_loss.item(), global_step)
                writer.add_scalar("Loss/GradientPenalty", gp.item(), global_step)
                writer.add_scalar("Score/D_Real", d_real.mean().item(), global_step)
                writer.add_scalar("Score/D_Fake", d_fake.mean().item(), global_step)

            global_step += 1

        # Epoch summary
        avg_d = np.mean(d_losses)
        avg_g = np.mean(g_losses)
        elapsed = time.time() - epoch_start

        print(
            f"Epoch [{epoch}/{epochs}] | "
            f"D_loss: {avg_d:.4f} | G_loss: {avg_g:.4f} | "
            f"Time: {elapsed:.1f}s"
        )

        writer.add_scalar("Epoch/D_loss", avg_d, epoch)
        writer.add_scalar("Epoch/G_loss", avg_g, epoch)

        # Save best model
        if avg_d < best_d_loss:
            best_d_loss = avg_d
            torch.save(
                {
                    "epoch": epoch,
                    "generator_state_dict": generator.state_dict(),
                    "discriminator_state_dict": discriminator.state_dict(),
                    "opt_g_state_dict": opt_g.state_dict(),
                    "opt_d_state_dict": opt_d.state_dict(),
                    "d_loss": avg_d,
                    "g_loss": avg_g,
                },
                out_path / "best_model.pt",
            )

        # Periodic checkpoint
        if epoch % 50 == 0:
            torch.save(
                {
                    "epoch": epoch,
                    "generator_state_dict": generator.state_dict(),
                    "discriminator_state_dict": discriminator.state_dict(),
                },
                out_path / f"checkpoint_epoch_{epoch}.pt",
            )

    # ------------------------------------------------------------------
    # Export to ONNX with INT8 quantization
    # ------------------------------------------------------------------
    print("[Shield-GAN] Exporting to ONNX...")

    generator.eval()

    # Create sample inputs for tracing
    sample_noise = torch.randn(1, NOISE_DIM, device=device)
    sample_type = torch.tensor([[1.0, 0.0, 0.0]], device=device)  # browsing

    # Export the full generator (combined output)
    class GeneratorONNXWrapper(nn.Module):
        """Wraps Generator for ONNX export — single forward producing
        concatenated output vector matching the Discriminator input."""

        def __init__(self, generator: Generator):
            super().__init__()
            self.generator = generator

        def forward(self, noise: torch.Tensor, traffic_type: torch.Tensor) -> torch.Tensor:
            pkt, iat, hist = self.generator(noise, traffic_type)
            # Denormalize for inference
            pkt = pkt * 1500.0
            iat = (torch.expm1(iat * 15.0))  # Reverse log1p normalization
            return torch.cat([pkt, iat, hist], dim=1)

    wrapper = GeneratorONNXWrapper(generator)
    wrapper.eval()

    onnx_path = out_path / "traffic_gan.onnx"

    torch.onnx.export(
        wrapper,
        (sample_noise, sample_type),
        str(onnx_path),
        input_names=["noise", "traffic_type"],
        output_names=["traffic_features"],
        dynamic_axes={
            "noise": {0: "batch_size"},
            "traffic_type": {0: "batch_size"},
            "traffic_features": {0: "batch_size"},
        },
        opset_version=17,
        do_constant_folding=True,
    )
    print(f"[Shield-GAN] ONNX model saved: {onnx_path}")

    # INT8 quantization via ONNX Runtime
    try:
        import onnxruntime as ort
        from onnxruntime.quantization import quantize_dynamic, QuantType

        quantized_path = out_path / "traffic_gan_int8.onnx"
        quantize_dynamic(
            model_input=str(onnx_path),
            model_output=str(quantized_path),
            weight_type=QuantType.QInt8,
        )
        print(f"[Shield-GAN] INT8 quantized model saved: {quantized_path}")
    except ImportError:
        print("[Shield-GAN] onnxruntime not installed — skipping INT8 quantization")
        print("  Install with: pip install onnxruntime")

    # Save final model
    torch.save(
        {
            "epoch": epochs,
            "generator_state_dict": generator.state_dict(),
            "discriminator_state_dict": discriminator.state_dict(),
        },
        out_path / "final_model.pt",
    )

    print("[Shield-GAN] Training complete!")
    writer.close()


# ---------------------------------------------------------------------------
# Synthetic data generation (for testing without real captures)
# ---------------------------------------------------------------------------


def generate_synthetic_data(output_dir: str, n_samples: int = 10000) -> None:
    """
    Generate synthetic training data for development/testing.
    Real deployments should use actual captured traffic.
    """
    out_path = pathlib.Path(output_dir)
    out_path.mkdir(parents=True, exist_ok=True)

    rng = np.random.default_rng(42)

    # Browsing: small packets, variable timing
    browsing_pkt = rng.exponential(scale=400, size=(n_samples // 3, MAX_PACKETS)).astype(np.float32)
    browsing_iat = rng.exponential(scale=50000, size=(n_samples // 3, MAX_PACKETS)).astype(np.float32)

    # Video: medium-large packets, consistent timing
    n_video = n_samples // 3
    video_pkt = rng.normal(loc=1200, scale=200, size=(n_video, MAX_PACKETS)).astype(np.float32)
    video_iat = rng.normal(loc=20000, scale=5000, size=(n_video, MAX_PACKETS)).astype(np.float32)

    # Download: large packets, consistent
    n_download = n_samples - (n_samples // 3) * 2
    download_pkt = rng.normal(loc=1400, scale=100, size=(n_download, MAX_PACKETS)).astype(np.float32)
    download_iat = rng.exponential(scale=1000, size=(n_download, MAX_PACKETS)).astype(np.float32)

    pkt_sizes = np.clip(
        np.concatenate([browsing_pkt, video_pkt, download_pkt], axis=0), 0, 1500
    )
    iat = np.clip(
        np.concatenate([browsing_iat, video_iat, download_iat], axis=0), 0, None
    )

    # Byte histograms: simulate TLS-like distributions
    histograms = np.zeros((n_samples, BYTE_HIST_DIM), dtype=np.float32)
    for i in range(n_samples):
        # TLS records have distinctive byte patterns
        base = rng.dirichlet(np.ones(BYTE_HIST_DIM))
        # Emphasize 0x00, 0x03 (TLS record type), 0x01, 0x16 (handshake)
        base[0x00] += 0.05
        base[0x03] += 0.03
        base[0x01] += 0.02
        base[0x16] += 0.02
        histograms[i] = base / base.sum()

    # Traffic type labels
    types = np.concatenate([
        np.zeros(n_samples // 3, dtype=np.int64),
        np.ones(n_video, dtype=np.int64),
        np.full(n_download, 2, dtype=np.int64),
    ])

    np.save(out_path / "packet_sizes.npy", pkt_sizes)
    np.save(out_path / "inter_arrival_times.npy", iat)
    np.save(out_path / "byte_histograms.npy", histograms)
    np.save(out_path / "traffic_types.npy", types)

    print(f"[Shield-GAN] Synthetic data saved to {out_path} ({n_samples} samples)")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(description="Train Adversarial Traffic GAN")
    parser.add_argument("--data-dir", type=str, required=True, help="Directory with training data (.npy files)")
    parser.add_argument("--epochs", type=int, default=500, help="Number of training epochs")
    parser.add_argument("--batch-size", type=int, default=64, help="Batch size")
    parser.add_argument("--lr-d", type=float, default=1e-4, help="Discriminator learning rate")
    parser.add_argument("--lr-g", type=float, default=1e-4, help="Generator learning rate")
    parser.add_argument("--n-critic", type=int, default=5, help="Critic updates per generator update")
    parser.add_argument("--lambda-gp", type=float, default=10.0, help="Gradient penalty coefficient")
    parser.add_argument("--output-dir", type=str, default="./models", help="Model output directory")
    parser.add_argument("--device", type=str, default="auto", help="Training device (auto/cuda/cpu)")
    parser.add_argument("--generate-synthetic", action="store_true", help="Generate synthetic data and exit")
    parser.add_argument("--synthetic-samples", type=int, default=10000, help="Number of synthetic samples")
    parser.add_argument("--export-onnx", type=str, default=None, help="Export path for ONNX model")

    args = parser.parse_args()

    if args.generate_synthetic:
        generate_synthetic_data(args.data_dir, args.synthetic_samples)
        return

    train(
        data_dir=args.data_dir,
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr_d=args.lr_d,
        lr_g=args.lr_g,
        n_critic=args.n_critic,
        lambda_gp=args.lambda_gp,
        output_dir=args.output_dir,
        device_str=args.device,
    )


if __name__ == "__main__":
    main()
