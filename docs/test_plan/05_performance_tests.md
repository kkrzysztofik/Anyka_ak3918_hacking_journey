# Performance Test Specifications

## Overview

Enhanced performance testing framework with statistical analysis, comprehensive metrics, and automated performance regression detection.

## Current Performance Test Issues

### Problems to Fix
- **Simplistic Metrics**: Only basic response time measurement
- **No Statistical Analysis**: Single measurement per operation
- **Missing Regression Detection**: No baseline comparison
- **Limited Coverage**: Only tests basic operations
- **No Resource Monitoring**: CPU, memory usage not tracked

## Enhanced Performance Framework

### Performance Metrics

```python
# utils/performance_metrics.py
"""Comprehensive performance metrics collection."""

import time
import psutil
import statistics
from dataclasses import dataclass, field
from typing import List, Dict, Any, Optional

@dataclass
class PerformanceMetrics:
    """Performance metrics for a single operation."""
    operation_name: str
    response_times: List[float] = field(default_factory=list)
    cpu_usage: List[float] = field(default_factory=list)
    memory_usage: List[float] = field(default_factory=list)
    error_count: int = 0
    success_count: int = 0

    @property
    def avg_response_time(self) -> float:
        """Average response time."""
        return statistics.mean(self.response_times) if self.response_times else 0.0

    @property
    def median_response_time(self) -> float:
        """Median response time."""
        return statistics.median(self.response_times) if self.response_times else 0.0

    @property
    def std_dev_response_time(self) -> float:
        """Standard deviation of response times."""
        return statistics.stdev(self.response_times) if len(self.response_times) > 1 else 0.0

    @property
    def p95_response_time(self) -> float:
        """95th percentile response time."""
        if not self.response_times:
            return 0.0
        sorted_times = sorted(self.response_times)
        index = int(0.95 * len(sorted_times))
        return sorted_times[index]

    @property
    def success_rate(self) -> float:
        """Success rate as percentage."""
        total = self.success_count + self.error_count
        return (self.success_count / total * 100) if total > 0 else 0.0

class PerformanceCollector:
    """Collects performance metrics during test execution."""

    def __init__(self, operation_name: str):
        self.metrics = PerformanceMetrics(operation_name)
        self.start_time: Optional[float] = None
        self.start_cpu: Optional[float] = None
        self.start_memory: Optional[float] = None

    def start_measurement(self):
        """Start measuring performance."""
        self.start_time = time.time()
        self.start_cpu = psutil.cpu_percent()
        self.start_memory = psutil.virtual_memory().percent

    def end_measurement(self, success: bool = True):
        """End measurement and record metrics."""
        if self.start_time is None:
            raise ValueError("Must call start_measurement first")

        end_time = time.time()
        response_time = end_time - self.start_time

        self.metrics.response_times.append(response_time)
        self.metrics.cpu_usage.append(psutil.cpu_percent() - self.start_cpu)
        self.metrics.memory_usage.append(psutil.virtual_memory().percent - self.start_memory)

        if success:
            self.metrics.success_count += 1
        else:
            self.metrics.error_count += 1

        # Reset for next measurement
        self.start_time = None
        self.start_cpu = None
        self.start_memory = None
```

### Performance Test Implementation

Create `tests/performance/test_device_performance.py`:

```python
"""Enhanced device service performance tests with statistical analysis."""

import pytest
import time
import concurrent.futures
from utils.performance_metrics import PerformanceCollector
from utils.performance_baselines import PerformanceBaseline

class TestONVIFDevicePerformance:
    """Comprehensive device service performance tests."""

    @pytest.mark.performance
    @pytest.mark.onvif_device
    def test_device_operations_response_time(self, device_client):
        """Test response times with statistical analysis."""
        operations = [
            ("GetDeviceInformation", device_client.get_device_information),
            ("GetCapabilities", device_client.get_capabilities),
            ("GetSystemDateAndTime", device_client.get_system_date_time),
            ("GetServices", lambda: device_client.get_services(include_capability=False))
        ]

        results = {}

        for op_name, op_func in operations:
            collector = PerformanceCollector(op_name)

            # Run multiple iterations for statistical significance
            for iteration in range(20):
                collector.start_measurement()

                try:
                    response = op_func()
                    success = response.status_code == 200
                except Exception as e:
                    success = False
                    print(f"Error in {op_name} iteration {iteration}: {e}")

                collector.end_measurement(success)

            results[op_name] = collector.metrics

            # Performance assertions
            assert collector.metrics.avg_response_time < 2.0, \
                f"{op_name} avg response time too slow: {collector.metrics.avg_response_time:.3f}s"

            assert collector.metrics.p95_response_time < 3.0, \
                f"{op_name} 95th percentile too slow: {collector.metrics.p95_response_time:.3f}s"

            assert collector.metrics.success_rate >= 95.0, \
                f"{op_name} success rate too low: {collector.metrics.success_rate:.1f}%"

            assert collector.metrics.std_dev_response_time < 1.0, \
                f"{op_name} response time too variable: {collector.metrics.std_dev_response_time:.3f}s"

        # Log detailed performance summary
        self._log_performance_summary(results)

    @pytest.mark.performance
    @pytest.mark.slow
    def test_concurrent_device_operations(self, device_client):
        """Test performance under concurrent load."""
        def execute_operation():
            """Execute a device operation."""
            collector = PerformanceCollector("ConcurrentGetDeviceInfo")
            collector.start_measurement()

            try:
                response = device_client.get_device_information()
                success = response.status_code == 200
            except Exception:
                success = False

            collector.end_measurement(success)
            return collector.metrics

        # Execute operations concurrently
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(execute_operation) for _ in range(50)]
            results = [future.result() for future in concurrent.futures.as_completed(futures)]

        # Aggregate results
        all_response_times = []
        success_count = 0
        error_count = 0

        for result in results:
            all_response_times.extend(result.response_times)
            success_count += result.success_count
            error_count += result.error_count

        avg_concurrent_time = sum(all_response_times) / len(all_response_times)
        concurrent_success_rate = (success_count / (success_count + error_count)) * 100

        # Performance assertions for concurrent access
        assert avg_concurrent_time < 5.0, \
            f"Concurrent avg response time too slow: {avg_concurrent_time:.3f}s"

        assert concurrent_success_rate >= 90.0, \
            f"Concurrent success rate too low: {concurrent_success_rate:.1f}%"

        print(f"Concurrent performance: {avg_concurrent_time:.3f}s avg, {concurrent_success_rate:.1f}% success rate")

    @pytest.mark.performance
    def test_memory_usage_stability(self, device_client):
        """Test memory usage remains stable during extended operation."""
        import psutil
        import gc

        initial_memory = psutil.virtual_memory().percent
        memory_readings = []

        # Execute operations continuously for 2 minutes
        start_time = time.time()
        operation_count = 0

        while time.time() - start_time < 120:  # 2 minutes
            try:
                response = device_client.get_device_information()
                if response.status_code == 200:
                    operation_count += 1

                # Record memory usage every 10 operations
                if operation_count % 10 == 0:
                    memory_readings.append(psutil.virtual_memory().percent)
                    gc.collect()  # Force garbage collection

            except Exception as e:
                print(f"Error during memory test: {e}")

            time.sleep(0.1)  # Brief pause between operations

        final_memory = psutil.virtual_memory().percent
        memory_increase = final_memory - initial_memory

        # Memory stability assertions
        assert memory_increase < 5.0, \
            f"Memory usage increased too much: {memory_increase:.1f}%"

        assert operation_count > 100, \
            f"Not enough operations completed: {operation_count}"

        print(f"Memory stability test: {operation_count} operations, {memory_increase:.1f}% memory increase")

    @pytest.mark.performance
    def test_throughput_measurement(self, device_client):
        """Measure operation throughput (operations per second)."""
        operation_count = 0
        start_time = time.time()
        test_duration = 30  # 30 seconds

        while time.time() - start_time < test_duration:
            try:
                response = device_client.get_system_date_time()
                if response.status_code == 200:
                    operation_count += 1
            except Exception as e:
                print(f"Throughput test error: {e}")

        actual_duration = time.time() - start_time
        throughput = operation_count / actual_duration

        # Throughput assertions
        assert throughput >= 5.0, f"Throughput too low: {throughput:.1f} ops/sec"

        print(f"Throughput: {throughput:.1f} operations/second ({operation_count} ops in {actual_duration:.1f}s)")

    def _log_performance_summary(self, results: Dict[str, Any]):
        """Log detailed performance summary."""
        print("\n" + "="*80)
        print("DEVICE SERVICE PERFORMANCE SUMMARY")
        print("="*80)

        for op_name, metrics in results.items():
            print(f"\n{op_name}:")
            print(f"  Average Response Time: {metrics.avg_response_time:.3f}s")
            print(f"  Median Response Time:  {metrics.median_response_time:.3f}s")
            print(f"  95th Percentile:       {metrics.p95_response_time:.3f}s")
            print(f"  Standard Deviation:    {metrics.std_dev_response_time:.3f}s")
            print(f"  Success Rate:          {metrics.success_rate:.1f}%")
            print(f"  Total Operations:      {metrics.success_count + metrics.error_count}")

        print("\n" + "="*80)
```

### Performance Baseline Management

Create `utils/performance_baselines.py`:

```python
"""Performance baseline management for regression detection."""

import json
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, Optional
from dataclasses import asdict

class PerformanceBaseline:
    """Manages performance baselines and regression detection."""

    def __init__(self, baseline_file: str = "performance_baselines.json"):
        self.baseline_file = Path(baseline_file)
        self.baselines = self._load_baselines()

    def _load_baselines(self) -> Dict[str, Any]:
        """Load existing baselines from file."""
        if self.baseline_file.exists():
            with open(self.baseline_file, 'r') as f:
                return json.load(f)
        return {}

    def save_baseline(self, operation_name: str, metrics: Dict[str, float]):
        """Save new baseline for operation."""
        baseline_data = {
            'avg_response_time': metrics['avg_response_time'],
            'p95_response_time': metrics['p95_response_time'],
            'success_rate': metrics['success_rate'],
            'created_at': datetime.now().isoformat(),
            'test_conditions': {
                'iterations': metrics.get('iterations', 20),
                'environment': 'test'
            }
        }

        self.baselines[operation_name] = baseline_data

        with open(self.baseline_file, 'w') as f:
            json.dump(self.baselines, f, indent=2)

    def check_regression(self, operation_name: str, current_metrics: Dict[str, float],
                        tolerance: float = 0.2) -> Dict[str, Any]:
        """Check for performance regression against baseline."""
        if operation_name not in self.baselines:
            return {'status': 'no_baseline', 'message': 'No baseline available'}

        baseline = self.baselines[operation_name]
        regression_detected = False
        issues = []

        # Check response time regression
        baseline_avg = baseline['avg_response_time']
        current_avg = current_metrics['avg_response_time']

        if current_avg > baseline_avg * (1 + tolerance):
            regression_detected = True
            increase = ((current_avg - baseline_avg) / baseline_avg) * 100
            issues.append(f"Response time increased by {increase:.1f}%")

        # Check P95 regression
        baseline_p95 = baseline['p95_response_time']
        current_p95 = current_metrics['p95_response_time']

        if current_p95 > baseline_p95 * (1 + tolerance):
            regression_detected = True
            increase = ((current_p95 - baseline_p95) / baseline_p95) * 100
            issues.append(f"P95 response time increased by {increase:.1f}%")

        # Check success rate regression
        baseline_success = baseline['success_rate']
        current_success = current_metrics['success_rate']

        if current_success < baseline_success * (1 - tolerance/10):  # 2% tolerance for success rate
            regression_detected = True
            decrease = baseline_success - current_success
            issues.append(f"Success rate decreased by {decrease:.1f}%")

        return {
            'status': 'regression' if regression_detected else 'passed',
            'issues': issues,
            'baseline_date': baseline.get('created_at', 'unknown'),
            'comparison': {
                'avg_response_time': {
                    'baseline': baseline_avg,
                    'current': current_avg,
                    'change_percent': ((current_avg - baseline_avg) / baseline_avg) * 100
                },
                'p95_response_time': {
                    'baseline': baseline_p95,
                    'current': current_p95,
                    'change_percent': ((current_p95 - baseline_p95) / baseline_p95) * 100
                },
                'success_rate': {
                    'baseline': baseline_success,
                    'current': current_success,
                    'change_percent': current_success - baseline_success
                }
            }
        }
```

### Performance Configuration

Create `config/performance_config.py`:

```python
"""Performance testing configuration."""

from dataclasses import dataclass
from typing import Dict, Any

@dataclass
class PerformanceThresholds:
    """Performance threshold configuration."""

    # Response time thresholds (seconds)
    max_avg_response_time: float = 2.0
    max_p95_response_time: float = 3.0
    max_std_dev_response_time: float = 1.0

    # Success rate thresholds (percentage)
    min_success_rate: float = 95.0
    min_concurrent_success_rate: float = 90.0

    # Resource usage thresholds
    max_memory_increase_percent: float = 5.0
    min_throughput_ops_per_sec: float = 5.0

    # Test configuration
    standard_iterations: int = 20
    concurrent_operations: int = 50
    concurrent_workers: int = 10
    memory_test_duration_sec: int = 120
    throughput_test_duration_sec: int = 30

# Environment-specific configurations
PERFORMANCE_CONFIG = {
    'development': PerformanceThresholds(
        max_avg_response_time=3.0,  # More relaxed for dev
        min_success_rate=90.0
    ),
    'testing': PerformanceThresholds(),  # Standard thresholds
    'production': PerformanceThresholds(
        max_avg_response_time=1.5,  # Stricter for production
        max_p95_response_time=2.0,
        min_success_rate=99.0
    )
}
```

## Performance Validation Scripts

Create `scripts/validate_performance_tests.py`:

```python
#!/usr/bin/env python3
"""Validate performance test implementation."""

import subprocess
import json
from pathlib import Path

def run_performance_tests():
    """Run performance tests and validate results."""
    result = subprocess.run([
        "pytest", "tests/performance/",
        "-m", "performance",
        "--json-report",
        "--json-report-file=reports/performance_results.json"
    ], capture_output=True, text=True)

    if result.returncode != 0:
        print("❌ Performance tests failed")
        print(result.stdout)
        print(result.stderr)
        return False

    # Validate performance report exists
    report_file = Path("reports/performance_results.json")
    if not report_file.exists():
        print("❌ Performance report not generated")
        return False

    with open(report_file, 'r') as f:
        report = json.load(f)

    # Check test results
    if report['summary']['failed'] > 0:
        print(f"❌ {report['summary']['failed']} performance tests failed")
        return False

    print(f"✅ All {report['summary']['passed']} performance tests passed")
    return True

def validate_baseline_management():
    """Validate performance baseline functionality."""
    from utils.performance_baselines import PerformanceBaseline

    baseline = PerformanceBaseline("test_baselines.json")

    # Test baseline save/load
    test_metrics = {
        'avg_response_time': 1.5,
        'p95_response_time': 2.0,
        'success_rate': 98.5
    }

    baseline.save_baseline("TestOperation", test_metrics)

    # Test regression detection
    current_metrics = {
        'avg_response_time': 2.0,  # 33% increase
        'p95_response_time': 2.2,
        'success_rate': 97.0
    }

    regression_result = baseline.check_regression("TestOperation", current_metrics)

    if regression_result['status'] != 'regression':
        print("❌ Regression detection not working")
        return False

    print("✅ Baseline management working correctly")

    # Cleanup test file
    Path("test_baselines.json").unlink(missing_ok=True)
    return True

def main():
    """Run all performance test validations."""
    print("🔍 Validating performance test implementation")

    validations = [
        run_performance_tests,
        validate_baseline_management
    ]

    passed = sum(1 for validation in validations if validation())
    total = len(validations)

    print(f"\n📊 Performance Validation Results: {passed}/{total} passed")
    return 0 if passed == total else 1

if __name__ == "__main__":
    exit(main())
```

## Implementation Checklist

- [ ] Create performance metrics utilities
- [ ] Implement enhanced performance tests
- [ ] Add baseline management system
- [ ] Create performance configuration
- [ ] Add validation scripts
- [ ] Run performance tests and verify results
- [ ] Set up performance regression detection

## Next Steps

After implementing performance testing framework, proceed to [06_compliance_framework.md](06_compliance_framework.md) for ONVIF specification compliance validation.

## Related Documentation

- **Previous**: [04_device_tests.md](04_device_tests.md) - Device test refactoring
- **Next**: [06_compliance_framework.md](06_compliance_framework.md) - ONVIF compliance validation
- **Issues Addressed**: [01_current_issues.md](01_current_issues.md#low-priority-issues) - Simplistic performance tests
- **Templates**:
  - [templates/test_template.py](templates/test_template.py) - Performance test patterns
  - [templates/fixture_template.py](templates/fixture_template.py) - Performance fixtures
- **Directory Structure**: [02_directory_structure.md](02_directory_structure.md#phase-1-create-new-structure) - `tests/performance/` location
- **Quality Standards**: [03_code_quality.md](03_code_quality.md) - Performance test quality requirements