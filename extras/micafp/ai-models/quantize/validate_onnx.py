#!/usr/bin/env python3
"""
ONNX Model Validation

Validates ONNX models for correctness, checks opset version,
verifies input/output shapes, and runs inference tests.
"""

import numpy as np
import onnx
import onnxruntime as ort
from pathlib import Path
import json
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ONNXValidator")


def validate_onnx_model(
    model_path: str,
    expected_input_shape: tuple | None = None,
    expected_output_shape: tuple | None = None,
    expected_opset: int = 17,
    test_inference: bool = True,
) -> dict:
    """
    Validate an ONNX model.

    Returns a dict with validation results.
    """

    results = {
        "model_path": model_path,
        "valid": True,
        "errors": [],
        "warnings": [],
        "info": {},
    }

    path = Path(model_path)
    if not path.exists():
        results["valid"] = False
        results["errors"].append(f"Model file not found: {model_path}")
        return results

    # Load and check ONNX model
    try:
        model = onnx.load(str(path))
        onnx.checker.check_model(model)
        logger.info(f"✓ ONNX model structure valid: {model_path}")
    except onnx.checker.ValidationError as e:
        results["valid"] = False
        results["errors"].append(f"ONNX validation error: {e}")
        return results
    except Exception as e:
        results["valid"] = False
        results["errors"].append(f"Failed to load model: {e}")
        return results

    # Check opset version
    opset_version = model.opset_import[0].version
    results["info"]["opset_version"] = opset_version

    if opset_version < expected_opset:
        results["warnings"].append(
            f"Opset version {opset_version} is below expected {expected_opset}"
        )
    else:
        logger.info(f"✓ Opset version: {opset_version}")

    # Check inputs
    input_info = model.graph.input[0]
    input_name = input_info.name
    input_shape = []
    for dim in input_info.type.tensor_type.shape.dim:
        if dim.dim_value > 0:
            input_shape.append(dim.dim_value)
        elif dim.dim_param:
            input_shape.append(dim.dim_param)
        else:
            input_shape.append(1)

    results["info"]["input_name"] = input_name
    results["info"]["input_shape"] = input_shape

    if expected_input_shape:
        if tuple(input_shape) != expected_input_shape:
            results["warnings"].append(
                f"Input shape {input_shape} differs from expected {expected_input_shape}"
            )

    # Check outputs
    output_info = model.graph.output[0]
    output_name = output_info.name
    output_shape = []
    for dim in output_info.type.tensor_type.shape.dim:
        if dim.dim_value > 0:
            output_shape.append(dim.dim_value)
        elif dim.dim_param:
            output_shape.append(dim.dim_param)
        else:
            output_shape.append(1)

    results["info"]["output_name"] = output_name
    results["info"]["output_shape"] = output_shape

    # Model size
    model_size_mb = path.stat().st_size / (1024 * 1024)
    results["info"]["size_mb"] = round(model_size_mb, 2)

    # Count parameters
    param_count = 0
    for initializer in model.graph.initializer:
        param_count += np.prod(initializer.dims)
    results["info"]["param_count"] = int(param_count)

    # Run inference test
    if test_inference:
        try:
            session = ort.InferenceSession(str(path))

            # Get actual input info from session
            sess_input = session.get_inputs()[0]
            actual_shape = sess_input.shape

            # Replace dynamic axes with 1
            test_shape = []
            for dim in actual_shape:
                if isinstance(dim, int) and dim > 0:
                    test_shape.append(dim)
                else:
                    test_shape.append(1)

            test_input = np.random.randn(*test_shape).astype(np.float32)
            output = session.run(None, {sess_input.name: test_input})

            results["info"]["inference_output_shape"] = list(output[0].shape)
            results["info"]["inference_output_dtype"] = str(output[0].dtype)

            # Check output values are valid (no NaN/Inf)
            if np.any(np.isnan(output[0])) or np.any(np.isinf(output[0])):
                results["valid"] = False
                results["errors"].append("Model produces NaN or Inf values")
            else:
                logger.info(f"✓ Inference test passed: input {test_shape} → output {output[0].shape}")

            # Benchmark inference time
            times = []
            for _ in range(50):
                import time
                start = time.perf_counter()
                session.run(None, {sess_input.name: test_input})
                times.append(time.perf_counter() - start)

            avg_time_ms = np.mean(times) * 1000
            std_time_ms = np.std(times) * 1000
            results["info"]["avg_inference_ms"] = round(avg_time_ms, 2)
            results["info"]["std_inference_ms"] = round(std_time_ms, 2)

            logger.info(
                f"✓ Inference benchmark: {avg_time_ms:.2f} ± {std_time_ms:.2f} ms"
            )

        except Exception as e:
            results["valid"] = False
            results["errors"].append(f"Inference test failed: {e}")

    return results


def validate_all_models(models_dir: str = "models") -> None:
    """Validate all ONNX models in a directory"""

    models_path = Path(models_dir)
    all_results = []

    for onnx_file in sorted(models_path.glob("*.onnx")):
        logger.info(f"\n{'='*60}")
        logger.info(f"Validating: {onnx_file.name}")
        logger.info(f"{'='*60}")

        # Determine expected shapes based on model name
        if "dpi_classifier" in onnx_file.name:
            expected_input = ("batch_size", 47)
            expected_output = ("batch_size", 8)
        elif "traffic_predictor" in onnx_file.name:
            expected_input = ("batch_size", "seq_len", 47)
            expected_output = ("batch_size", 1)
        else:
            expected_input = None
            expected_output = None

        result = validate_onnx_model(
            str(onnx_file),
            expected_input_shape=expected_input,
            expected_output_shape=expected_output,
        )

        all_results.append(result)

        status = "✓ PASS" if result["valid"] else "✗ FAIL"
        logger.info(f"\n{status}: {onnx_file.name}")
        if result["errors"]:
            for err in result["errors"]:
                logger.error(f"  Error: {err}")
        if result["warnings"]:
            for warn in result["warnings"]:
                logger.warning(f"  Warning: {warn}")

    # Save validation report
    report_path = models_path / "validation_report.json"
    with open(report_path, "w") as f:
        json.dump(all_results, f, indent=2, default=str)

    # Summary
    passed = sum(1 for r in all_results if r["valid"])
    total = len(all_results)
    logger.info(f"\n{'='*60}")
    logger.info(f"Validation Summary: {passed}/{total} models passed")
    logger.info(f"Report saved to: {report_path}")


if __name__ == "__main__":
    validate_all_models()
