#!/usr/bin/env python3
"""
Master test runner for all Spotify Downloader tests
"""

import sys
import os
import subprocess
import time
from pathlib import Path

def run_python_test(test_file, test_name):
    """Run a Python test file"""
    print(f"\n{'='*60}")
    print(f"🧪 Running {test_name}")
    print(f"{'='*60}")
    
    try:
        result = subprocess.run([
            sys.executable, str(test_file)
        ], capture_output=True, text=True, timeout=300)  # 5 minute timeout
        
        if result.returncode == 0:
            print(f"✅ {test_name} PASSED")
            if result.stdout:
                print("Output:")
                print(result.stdout)
            return True
        else:
            print(f"❌ {test_name} FAILED")
            if result.stderr:
                print("Error:")
                print(result.stderr)
            if result.stdout:
                print("Output:")
                print(result.stdout)
            return False
            
    except subprocess.TimeoutExpired:
        print(f"⏰ {test_name} TIMED OUT")
        return False
    except Exception as e:
        print(f"❌ {test_name} ERROR: {e}")
        return False

def run_rust_tests():
    """Run Rust tests"""
    print(f"\n{'='*60}")
    print("🦀 Running Rust Tests")
    print(f"{'='*60}")
    
    try:
        # Change to src-tauri directory
        src_tauri_dir = Path(__file__).parent.parent / "src-tauri"
        
        result = subprocess.run([
            "cargo", "test", "--", "--nocapture"
        ], cwd=src_tauri_dir, capture_output=True, text=True, timeout=300)
        
        if result.returncode == 0:
            print("✅ Rust tests PASSED")
            if result.stdout:
                print("Output:")
                print(result.stdout)
            return True
        else:
            print("❌ Rust tests FAILED")
            if result.stderr:
                print("Error:")
                print(result.stderr)
            if result.stdout:
                print("Output:")
                print(result.stdout)
            return False
            
    except subprocess.TimeoutExpired:
        print("⏰ Rust tests TIMED OUT")
        return False
    except Exception as e:
        print(f"❌ Rust tests ERROR: {e}")
        return False

def main():
    """Run all tests"""
    print("🚀 Starting Spotify Downloader Test Suite")
    print("=" * 60)
    
    # Get test directory
    test_dir = Path(__file__).parent
    
    # Define test suites
    test_suites = [
        {
            "name": "Python Component Tests",
            "tests": [
                ("Audio Processor", test_dir / "components" / "python" / "test_audio_processor.py"),
                ("Spotify Integration", test_dir / "components" / "python" / "test_spotify_integration.py"),
            ]
        },
        {
            "name": "Integration Tests", 
            "tests": [
                ("End-to-End", test_dir / "components" / "integration" / "test_end_to_end.py"),
                ("Rust-Python Integration", test_dir / "components" / "integration" / "test_rust_python_integration.py"),
            ]
        }
    ]
    
    # Track results
    total_tests = 0
    passed_tests = 0
    failed_tests = []
    
    # Run Python tests
    for suite in test_suites:
        print(f"\n📋 {suite['name']}")
        print("-" * 40)
        
        for test_name, test_file in suite["tests"]:
            if test_file.exists():
                total_tests += 1
                if run_python_test(test_file, test_name):
                    passed_tests += 1
                else:
                    failed_tests.append(f"{suite['name']} - {test_name}")
            else:
                print(f"⚠️ Test file not found: {test_file}")
    
    # Run Rust tests
    print(f"\n📋 Rust Component Tests")
    print("-" * 40)
    total_tests += 1
    if run_rust_tests():
        passed_tests += 1
    else:
        failed_tests.append("Rust Component Tests")
    
    # Print summary
    print(f"\n{'='*60}")
    print("📊 TEST SUMMARY")
    print(f"{'='*60}")
    print(f"Total tests: {total_tests}")
    print(f"Passed: {passed_tests}")
    print(f"Failed: {len(failed_tests)}")
    print(f"Success rate: {(passed_tests/total_tests)*100:.1f}%")
    
    if failed_tests:
        print(f"\n❌ Failed tests:")
        for test in failed_tests:
            print(f"   - {test}")
    else:
        print(f"\n🎉 All tests passed!")
    
    print(f"\n{'='*60}")
    print("🏁 Test suite completed!")
    
    return len(failed_tests) == 0

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
