
import os
import sys
import time
import requests
import json

# Add backend directory to path
sys.path.append(os.path.join(os.path.dirname(__file__), 'app'))

BASE_URL = "http://localhost:5001/api"

def print_result(name, success, message=""):
    status = "✅ PASS" if success else "❌ FAIL"
    print(f"{status} - {name}: {message}")

def test_runtime_control():
    print("=== Testing Runtime Control (Phase 7) ===")
    
    # 1. Create a Simulation (Mocking Project ID)
    # Assuming project 'proj_test' exists or we can reuse one. 
    # Actually, let's just check if we can reach the endpoints first.
    
    # We need a running simulation to test this properly.
    # This might be hard to test fully automated without a real long-running simulation.
    # But we can test the 'inject' endpoint response structure at least.
    
    # Let's try to pause a non-existent simulation to check error handling
    try:
        response = requests.post(f"{BASE_URL}/simulation/sim_fake_123/pause")
        if response.status_code == 404:
             print_result("Pause Non-Existent", True, "Correctly returned 404")
        else:
             print_result("Pause Non-Existent", False, f"Unexpected status: {response.status_code}")
    except Exception as e:
        print_result("Pause Non-Existent", False, str(e))

    # Test Inject on non-existent
    try:
        response = requests.post(f"{BASE_URL}/simulation/sim_fake_123/inject", json={"event_text": "Test Event"})
        if response.status_code == 404:
             print_result("Inject Non-Existent", True, "Correctly returned 404")
        else:
             print_result("Inject Non-Existent", False, f"Unexpected status: {response.status_code}")
    except Exception as e:
        print_result("Inject Non-Existent", False, str(e))

if __name__ == "__main__":
    test_runtime_control()
