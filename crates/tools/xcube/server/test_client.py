#!/usr/bin/env python3
"""
Simple test client for XCube inference server.

Usage:
    python test_client.py
"""

import requests
import json
import time


def test_health(base_url: str = "http://localhost:8000"):
    """Test the /health endpoint"""
    print("Testing /health endpoint...")
    try:
        response = requests.get(f"{base_url}/health", timeout=5)
        response.raise_for_status()

        data = response.json()
        print(f"  Status: {data['status']}")
        print(f"  XCube available: {data['xcube_available']}")
        print(f"  GPU available: {data['gpu_available']}")
        print(f"  GPU name: {data.get('gpu_name', 'N/A')}")
        print(f"  Model loaded: {data['model_loaded']}")

        if data.get('error'):
            print(f"  Error: {data['error']}")

        return data['status'] == 'ready'

    except requests.exceptions.ConnectionError:
        print("  ✗ Connection failed - is the server running?")
        return False
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False


def test_generate(
    prompt: str = "a wooden chair",
    base_url: str = "http://localhost:8000",
    use_fine: bool = False,
    ddim_steps: int = 50
):
    """Test the /generate endpoint"""
    print(f"\nTesting /generate endpoint with prompt: '{prompt}'...")

    request_data = {
        "prompt": prompt,
        "ddim_steps": ddim_steps,
        "guidance_scale": 7.5,
        "seed": 42,  # Fixed seed for reproducibility
        "use_fine": use_fine
    }

    try:
        print(f"  Request: {json.dumps(request_data, indent=2)}")
        print("  Waiting for inference (this may take 10-60 seconds)...")

        start_time = time.time()
        response = requests.post(
            f"{base_url}/generate",
            json=request_data,
            timeout=300  # 5 minute timeout for inference
        )
        elapsed = time.time() - start_time

        response.raise_for_status()

        result = response.json()

        print(f"\n  ✓ Inference completed in {elapsed:.1f} seconds")
        print(f"  Coarse points: {len(result['coarse_xyz'])}")
        print(f"  Coarse normals: {len(result['coarse_normal'])}")

        if result.get('fine_xyz'):
            print(f"  Fine points: {len(result['fine_xyz'])}")
            print(f"  Fine normals: {len(result['fine_normal'])}")
        else:
            print("  Fine resolution: not generated")

        # Optionally save to file
        output_file = "test_output.json"
        with open(output_file, 'w') as f:
            json.dump(result, f, indent=2)
        print(f"\n  Saved result to {output_file}")

        return True

    except requests.exceptions.Timeout:
        print("  ✗ Request timed out - inference may be too slow")
        return False
    except requests.exceptions.HTTPError as e:
        print(f"  ✗ HTTP error: {e}")
        if e.response is not None:
            print(f"  Response: {e.response.text}")
        return False
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False


def main():
    """Run all tests"""
    print("=" * 60)
    print("XCube Inference Server Test Client")
    print("=" * 60)

    base_url = "http://localhost:8000"

    # Test health endpoint
    is_ready = test_health(base_url)

    if not is_ready:
        print("\n⚠ Server is not ready. Please wait for models to load or check for errors.")
        print("  Run this script again when the server status is 'ready'.")
        return

    print("\n✓ Server is ready for inference")

    # Test generate endpoint (coarse only, fast)
    print("\n" + "=" * 60)
    print("Test 1: Quick inference (coarse only, 50 steps)")
    print("=" * 60)
    test_generate(
        prompt="a simple wooden chair",
        base_url=base_url,
        use_fine=False,
        ddim_steps=50
    )

    # Uncomment to test fine generation (slower)
    # print("\n" + "=" * 60)
    # print("Test 2: High-quality inference (coarse + fine, 100 steps)")
    # print("=" * 60)
    # test_generate(
    #     prompt="a detailed vintage red sports car",
    #     base_url=base_url,
    #     use_fine=True,
    #     ddim_steps=100
    # )

    print("\n" + "=" * 60)
    print("All tests completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
