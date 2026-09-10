#!/usr/bin/env python3
"""
UnifiedShield Traffic Predictor Training — PyTorch LSTM

Architecture: 2-layer LSTM (h=128) → FC → 1 (BCE loss)
Predicts whether a network flow will be blocked/throttled.
Exports to ONNX format.
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
logger = logging.getLogger("TrafficPredictor")

# ──────────────── Model Architecture ────────────────

class TrafficPredictor(nn.Module):
    """
    LSTM-based traffic predictor.

    Input: sequence of flow features (seq_len x feature_dim)
    Output: probability of block/throttle (0-1)

    Architecture:
        Embedding: feature_dim → 64
        LSTM: 2 layers, hidden_size=128, bidirectional
        Attention: self-attention over sequence
        FC: 256 → 128 → 1
    """

    def __init__(
        self,
        feature_dim: int = 47,
        hidden_size: int = 128,
        num_layers: int = 2,
        dropout: float = 0.3,
    ):
        super().__init__()

        self.feature_dim = feature_dim
        self.hidden_size = hidden_size
        self.num_layers = num_layers

        # Input embedding
        self.embedding = nn.Sequential(
            nn.Linear(feature_dim, 64),
            nn.LayerNorm(64),
            nn.GELU(),
            nn.Dropout(dropout),
        )

        # LSTM
        self.lstm = nn.LSTM(
            input_size=64,
            hidden_size=hidden_size,
            num_layers=num_layers,
            batch_first=True,
            bidirectional=True,
            dropout=dropout if num_layers > 1 else 0,
        )

        # Self-attention
        self.attention = nn.Sequential(
            nn.Linear(hidden_size * 2, 64),
            nn.Tanh(),
            nn.Linear(64, 1),
        )

        # Classification head
        self.classifier = nn.Sequential(
            nn.Linear(hidden_size * 2, 256),
            nn.LayerNorm(256),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(256, 128),
            nn.LayerNorm(128),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(128, 1),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """
        Args:
            x: (batch, seq_len, feature_dim)
        Returns:
            (batch, 1) — probability of block/throttle
        """
        # Embed
        embedded = self.embedding(x)  # (batch, seq_len, 64)

        # LSTM
        lstm_out, _ = self.lstm(embedded)  # (batch, seq_len, hidden*2)

        # Self-attention
        attn_weights = self.attention(lstm_out)  # (batch, seq_len, 1)
        attn_weights = torch.softmax(attn_weights, dim=1)
        context = (lstm_out * attn_weights).sum(dim=1)  # (batch, hidden*2)

        # Classify
        logits = self.classifier(context)  # (batch, 1)
        return logits

    def predict_probability(self, x: torch.Tensor) -> torch.Tensor:
        """Return sigmoid probability"""
        return torch.sigmoid(self.forward(x))


# ──────────────── Training ────────────────

def train(
    data_path: str = "data/traffic_dataset.npz",
    epochs: int = 100,
    batch_size: int = 128,
    lr: float = 1e-3,
    weight_decay: float = 0.01,
    seq_len: int = 10,
    val_split: float = 0.15,
    output_dir: str = "models",
):
    """Train the traffic predictor"""

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    logger.info(f"Training on {device}")

    # Load dataset
    data = np.load(data_path)
    X = torch.tensor(data["X"], dtype=torch.float32)
    y = torch.tensor(data["y"], dtype=torch.float32)

    # Reshape for LSTM: (samples, seq_len, features)
    n_samples = X.shape[0]
    feature_dim = X.shape[1]
    X = X.view(n_samples, 1, feature_dim)

    # If we have sequential data, create windows
    if seq_len > 1:
        X_seq = []
        y_seq = []
        for i in range(len(X) - seq_len + 1):
            X_seq.append(X[i : i + seq_len].squeeze(1))
            y_seq.append(y[i + seq_len - 1])
        X = torch.stack(X_seq)
        y = torch.stack(y_seq)
    else:
        X = X.expand(-1, seq_len, -1)
        y = y.unsqueeze(1)

    logger.info(f"Dataset: {X.shape[0]} samples, seq_len={X.shape[1]}, features={X.shape[2]}")

    # Split
    dataset = TensorDataset(X, y)
    val_size = int(len(dataset) * val_split)
    train_size = len(dataset) - val_size
    train_ds, val_ds = random_split(dataset, [train_size, val_size])

    train_loader = DataLoader(train_ds, batch_size=batch_size, shuffle=True, num_workers=4)
    val_loader = DataLoader(val_ds, batch_size=batch_size, shuffle=False, num_workers=4)

    # Model
    model = TrafficPredictor(feature_dim=feature_dim).to(device)
    total_params = sum(p.numel() for p in model.parameters())
    logger.info(f"Model parameters: {total_params:,}")

    # Optimizer
    optimizer = optim.AdamW(model.parameters(), lr=lr, weight_decay=weight_decay)
    scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=epochs, eta_min=1e-6)

    # BCE loss
    criterion = nn.BCEWithLogitsLoss()

    # Training loop
    best_val_loss = float("inf")
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

            torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
            optimizer.step()

            train_loss += loss.item() * batch_x.size(0)
            preds = (torch.sigmoid(logits) > 0.5).float()
            train_correct += (preds == batch_y).sum().item()
            train_total += batch_y.numel()

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
                preds = (torch.sigmoid(logits) > 0.5).float()
                val_correct += (preds == batch_y).sum().item()
                val_total += batch_y.numel()

        train_acc = train_correct / train_total
        val_acc = val_correct / val_total
        avg_train_loss = train_loss / train_total
        avg_val_loss = val_loss / val_total

        if (epoch + 1) % 5 == 0 or epoch == 0:
            logger.info(
                f"Epoch {epoch+1:3d}/{epochs} | "
                f"Train Loss: {avg_train_loss:.4f} Acc: {train_acc:.4f} | "
                f"Val Loss: {avg_val_loss:.4f} Acc: {val_acc:.4f}"
            )

        if avg_val_loss < best_val_loss:
            best_val_loss = avg_val_loss
            torch.save(model.state_dict(), output_path / "traffic_predictor_best.pt")

    logger.info(f"Best validation loss: {best_val_loss:.4f}")

    # Export ONNX
    export_onnx(model, output_path / "traffic_predictor.onnx", device, seq_len, feature_dim)

    # Save metadata
    metadata = {
        "model": "TrafficPredictor",
        "version": "2.0.0",
        "feature_dim": feature_dim,
        "hidden_size": 128,
        "num_layers": 2,
        "seq_len": seq_len,
        "best_val_loss": float(best_val_loss),
    }
    with open(output_path / "traffic_predictor_meta.json", "w") as f:
        json.dump(metadata, f, indent=2)

    return model


def export_onnx(
    model: nn.Module,
    path: str,
    device: torch.device,
    seq_len: int,
    feature_dim: int,
):
    """Export to ONNX opset 17"""
    model.eval()
    dummy_input = torch.randn(1, seq_len, feature_dim).to(device)

    torch.onnx.export(
        model,
        dummy_input,
        path,
        opset_version=17,
        input_names=["sequence"],
        output_names=["logits"],
        dynamic_axes={
            "sequence": {0: "batch_size", 1: "seq_len"},
            "logits": {0: "batch_size"},
        },
    )

    onnx_model = onnx.load(path)
    onnx.checker.check_model(onnx_model)
    logger.info(f"ONNX model exported to {path}")

    session = ort.InferenceSession(path)
    result = session.run(None, {"sequence": dummy_input.cpu().numpy()})
    logger.info(f"ONNX Runtime verification: output shape {result[0].shape}")


if __name__ == "__main__":
    train()
