#!/usr/bin/env python3
"""
UnifiedShield DPI Classifier Training — PyTorch

Architecture: 47 → 256 → 256 → 256 → 8 (residual connections)
Optimizer: AdamW 3e-4
Scheduler: Cosine annealing, 200 epochs
Export: ONNX opset 17, then INT8 quantized
"""

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader, TensorDataset, random_split
import numpy as np
import onnx
import onnxruntime as ort
from pathlib import Path
import json
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("DPI_Classifier")

# ──────────────── Model Architecture ────────────────

class ResidualBlock(nn.Module):
    """Residual block with LayerNorm and dropout"""

    def __init__(self, dim: int, dropout: float = 0.2):
        super().__init__()
        self.norm1 = nn.LayerNorm(dim)
        self.linear1 = nn.Linear(dim, dim)
        self.norm2 = nn.LayerNorm(dim)
        self.linear2 = nn.Linear(dim, dim)
        self.dropout = nn.Dropout(dropout)
        self.act = nn.GELU()

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        residual = x
        out = self.norm1(x)
        out = self.act(self.linear1(out))
        out = self.dropout(out)
        out = self.norm2(out)
        out = self.linear2(out)
        out = self.dropout(out)
        return self.act(out + residual)


class DPIClassifier(nn.Module):
    """
    DPI Signature Classifier

    Input: 47 features per packet/flow sample
    Output: 8 classes (normal, tls_rst, http_403, dns_poison,
                       sni_filter, protocol_detect, throttling, other_dpi)

    Architecture: 47 → 256x3 (residual) → 8
    """

    NUM_FEATURES = 47
    NUM_CLASSES = 8
    HIDDEN_DIM = 256

    CLASS_NAMES = [
        "normal",           # 0 - Normal traffic, no DPI detected
        "tls_rst",          # 1 - FAVA TLS reset (95-320ms)
        "http_403",         # 2 - HTTP 403 block
        "dns_poison",       # 3 - DNS poisoning (10.10.34.34/35)
        "sni_filter",       # 4 - SNI-based filtering
        "protocol_detect",  # 5 - Protocol detection (VPN/Tor)
        "throttling",       # 6 - Bandwidth throttling
        "other_dpi",        # 7 - Other DPI behavior
    ]

    def __init__(self, dropout: float = 0.2):
        super().__init__()

        # Input projection
        self.input_proj = nn.Sequential(
            nn.Linear(self.NUM_FEATURES, self.HIDDEN_DIM),
            nn.LayerNorm(self.HIDDEN_DIM),
            nn.GELU(),
            nn.Dropout(dropout),
        )

        # Residual blocks
        self.res_block1 = ResidualBlock(self.HIDDEN_DIM, dropout)
        self.res_block2 = ResidualBlock(self.HIDDEN_DIM, dropout)
        self.res_block3 = ResidualBlock(self.HIDDEN_DIM, dropout)

        # Classification head
        self.classifier = nn.Sequential(
            nn.LayerNorm(self.HIDDEN_DIM),
            nn.Linear(self.HIDDEN_DIM, 128),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(128, self.NUM_CLASSES),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = self.input_proj(x)
        x = self.res_block1(x)
        x = self.res_block2(x)
        x = self.res_block3(x)
        return self.classifier(x)


# ──────────────── Training ────────────────

def train(
    data_path: str = "data/dpi_dataset.npz",
    epochs: int = 200,
    batch_size: int = 256,
    lr: float = 3e-4,
    weight_decay: float = 0.01,
    val_split: float = 0.15,
    output_dir: str = "models",
):
    """Train the DPI classifier"""

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    logger.info(f"Training on {device}")

    # Load dataset
    data = np.load(data_path)
    X = torch.tensor(data["X"], dtype=torch.float32)
    y = torch.tensor(data["y"], dtype=torch.long)

    logger.info(f"Dataset: {X.shape[0]} samples, {X.shape[1]} features, {len(torch.unique(y))} classes")

    # Split
    dataset = TensorDataset(X, y)
    val_size = int(len(dataset) * val_split)
    train_size = len(dataset) - val_size
    train_ds, val_ds = random_split(dataset, [train_size, val_size])

    train_loader = DataLoader(train_ds, batch_size=batch_size, shuffle=True, num_workers=4)
    val_loader = DataLoader(val_ds, batch_size=batch_size, shuffle=False, num_workers=4)

    # Model
    model = DPIClassifier().to(device)
    total_params = sum(p.numel() for p in model.parameters())
    logger.info(f"Model parameters: {total_params:,}")

    # Optimizer & scheduler
    optimizer = optim.AdamW(model.parameters(), lr=lr, weight_decay=weight_decay)
    scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=epochs, eta_min=1e-6)

    # Loss with class weighting
    class_counts = torch.bincount(y)
    class_weights = 1.0 / (class_counts.float() + 1e-6)
    class_weights = class_weights / class_weights.sum() * len(class_counts)
    criterion = nn.CrossEntropyLoss(weight=class_weights.to(device))

    # Training loop
    best_val_acc = 0.0
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    for epoch in range(epochs):
        # Train
        model.train()
        train_loss = 0.0
        train_correct = 0
        train_total = 0

        for batch_x, batch_y in train_loader:
            batch_x, batch_y = batch_x.to(device), batch_y.to(device)

            optimizer.zero_grad()
            logits = model(batch_x)
            loss = criterion(logits, batch_y)
            loss.backward()

            # Gradient clipping
            torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)

            optimizer.step()

            train_loss += loss.item() * batch_x.size(0)
            train_correct += (logits.argmax(dim=1) == batch_y).sum().item()
            train_total += batch_x.size(0)

        scheduler.step()

        # Validate
        model.eval()
        val_loss = 0.0
        val_correct = 0
        val_total = 0

        with torch.no_grad():
            for batch_x, batch_y in val_loader:
                batch_x, batch_y = batch_x.to(device), batch_y.to(device)
                logits = model(batch_x)
                loss = criterion(logits, batch_y)

                val_loss += loss.item() * batch_x.size(0)
                val_correct += (logits.argmax(dim=1) == batch_y).sum().item()
                val_total += batch_x.size(0)

        train_acc = train_correct / train_total
        val_acc = val_correct / val_total
        avg_train_loss = train_loss / train_total
        avg_val_loss = val_loss / val_total

        if (epoch + 1) % 10 == 0 or epoch == 0:
            logger.info(
                f"Epoch {epoch+1:3d}/{epochs} | "
                f"Train Loss: {avg_train_loss:.4f} Acc: {train_acc:.4f} | "
                f"Val Loss: {avg_val_loss:.4f} Acc: {val_acc:.4f} | "
                f"LR: {scheduler.get_last_lr()[0]:.2e}"
            )

        # Save best model
        if val_acc > best_val_acc:
            best_val_acc = val_acc
            torch.save(model.state_dict(), output_path / "dpi_classifier_best.pt")

    logger.info(f"Best validation accuracy: {best_val_acc:.4f}")

    # Export ONNX
    export_onnx(model, output_path / "dpi_classifier.onnx", device)

    # Save metadata
    metadata = {
        "model": "DPIClassifier",
        "version": "2.0.0",
        "num_features": 47,
        "num_classes": 8,
        "class_names": DPIClassifier.CLASS_NAMES,
        "hidden_dim": 256,
        "best_val_acc": float(best_val_acc),
        "epochs": epochs,
    }
    with open(output_path / "dpi_classifier_meta.json", "w") as f:
        json.dump(metadata, f, indent=2)

    return model


def export_onnx(model: nn.Module, path: str, device: torch.device):
    """Export model to ONNX opset 17"""
    model.eval()
    dummy_input = torch.randn(1, 47).to(device)

    torch.onnx.export(
        model,
        dummy_input,
        path,
        opset_version=17,
        input_names=["features"],
        output_names=["logits"],
        dynamic_axes={
            "features": {0: "batch_size"},
            "logits": {0: "batch_size"},
        },
    )

    # Validate
    onnx_model = onnx.load(path)
    onnx.checker.check_model(onnx_model)
    logger.info(f"ONNX model exported to {path}")

    # Verify with ONNX Runtime
    session = ort.InferenceSession(path)
    result = session.run(None, {"features": dummy_input.cpu().numpy()})
    logger.info(f"ONNX Runtime verification: output shape {result[0].shape}")


if __name__ == "__main__":
    train()
