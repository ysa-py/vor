#!/usr/bin/env python3
"""
UnifiedShield Model Quantization — INT8 Quantization

Quantizes ONNX models to INT8 for faster inference in the browser
using ONNX Runtime Web with WebAssembly backend.
"""

import numpy as np
import onnx
from onnxruntime.quantization import (
    quantize_static,
    quantize_dynamic,
    QuantType,
    CalibrationDataReader,
)
from pathlib import Path
import json
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("Quantizer")


class RandomCalibrationDataReader(CalibrationDataReader):
    """Generate random calibration data for static quantization"""

    def __init__(self, model_path: str, num_samples: int = 100):
        self.num_samples = num_samples
        self.sample_count = 0

        # Read model input info
        model = onnx.load(model_path)
        input_info = model.graph.input[0]
        self.input_name = input_info.name

        # Get shape
        shape = []
        for dim in input_info.type.tensor_type.shape.dim:
            if dim.dim_value > 0:
                shape.append(dim.dim_value)
            else:
                shape.append(1)  # dynamic axis

        self.input_shape = shape
        self.model_path = model_path

    def get_next(self):
        if self.sample_count >= self.num_samples:
            return None

        self.sample_count += 1
        data = np.random.randn(*self.input_shape).astype(np.float32)
        return {self.input_name: data}

    def rewind(self):
        self.sample_count = 0


def quantize_model_int8(
    input_path: str,
    output_path: str,
    calibration_data_path: str | None = None,
    num_calibration_samples: int = 200,
    per_channel: bool = True,
) -> None:
    """
    Quantize an ONNX model to INT8.

    Args:
        input_path: Path to FP32 ONNX model
        output_path: Path to save INT8 ONNX model
        calibration_data_path: Optional path to calibration dataset
        num_calibration_samples: Number of calibration samples
        per_channel: Whether to use per-channel quantization
    """

    input_path = str(Path(input_path).resolve())
    output_path = str(Path(output_path).resolve())

    logger.info(f"Quantizing {input_path} → {output_path}")

    # Check model size before quantization
    original_size = Path(input_path).stat().st_size / (1024 * 1024)
    logger.info(f"Original model size: {original_size:.2f} MB")

    if calibration_data_path:
        # Static quantization with real calibration data
        data = np.load(calibration_data_path)
        X = data["X"] if "X" in data else data[list(data.keys())[0]]

        class CalibrationReader(CalibrationDataReader):
            def __init__(self, data, input_name):
                self.data = data
                self.input_name = input_name
                self.index = 0

            def get_next(self):
                if self.index >= len(self.data):
                    return None
                sample = self.data[self.index : self.index + 1].astype(np.float32)
                self.index += 1
                return {self.input_name: sample}

            def rewind(self):
                self.index = 0

        model = onnx.load(input_path)
        input_name = model.graph.input[0].name
        cal_reader = CalibrationReader(X[:num_calibration_samples], input_name)

        quantize_static(
            input_path,
            output_path,
            cal_reader,
            quant_format=onnxruntime.quantization.QuantFormat.QDQ,
            weight_type=QuantType.QInt8,
            per_channel=per_channel,
            nodes_to_exclude=[],  # Quantize all nodes
        )
    else:
        # Dynamic quantization (no calibration data needed)
        logger.info("Using dynamic quantization (no calibration data)")

        quantize_dynamic(
            input_path,
            output_path,
            weight_type=QuantType.QInt8,
            per_channel=per_channel,
            extra_options={
                "ActivationSymmetric": True,
                "WeightSymmetric": True,
            },
        )

    # Report results
    quantized_size = Path(output_path).stat().st_size / (1024 * 1024)
    logger.info(f"Quantized model size: {quantized_size:.2f} MB")
    logger.info(f"Size reduction: {(1 - quantized_size / original_size) * 100:.1f}%")

    # Validate quantized model
    try:
        import onnxruntime as ort

        session = ort.InferenceSession(output_path)
        dummy_input = np.random.randn(1, 47).astype(np.float32)
        input_name = session.get_inputs()[0].name
        result = session.run(None, {input_name: dummy_input})
        logger.info(f"Validation: output shape = {result[0].shape}")
    except Exception as e:
        logger.warning(f"Validation failed: {e}")

    # Save quantization metadata
    meta = {
        "input_path": input_path,
        "output_path": output_path,
        "original_size_mb": round(original_size, 2),
        "quantized_size_mb": round(quantized_size, 2),
        "reduction_pct": round((1 - quantized_size / original_size) * 100, 1),
        "quantization_type": "static" if calibration_data_path else "dynamic",
        "weight_type": "int8",
        "per_channel": per_channel,
    }

    meta_path = Path(output_path).with_suffix(".quant_meta.json")
    with open(meta_path, "w") as f:
        json.dump(meta, f, indent=2)

    logger.info(f"Quantization metadata saved to {meta_path}")


def quantize_all_models(models_dir: str = "models") -> None:
    """Quantize all ONNX models in the directory"""

    models_path = Path(models_dir)

    for onnx_file in models_path.glob("*.onnx"):
        output_file = onnx_file.with_name(
            onnx_file.stem + "_int8" + onnx_file.suffix
        )

        # Look for calibration data
        cal_data = models_path / "dpi_dataset.npz"

        quantize_model_int8(
            str(onnx_file),
            str(output_file),
            calibration_data_path=str(cal_data) if cal_data.exists() else None,
        )


if __name__ == "__main__":
    quantize_all_models()
