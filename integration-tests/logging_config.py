"""
Logging configuration for ONVIF integration tests
"""
import logging
import logging.handlers
import os
from datetime import datetime


def setup_test_logging(log_level: str = "DEBUG", log_dir: str = "reports") -> None:
    """
    Set up comprehensive logging for integration tests

    Args:
        log_level: Logging level (DEBUG, INFO, WARNING, ERROR)
        log_dir: Directory to store log files
    """
    # Create log directory if it doesn't exist
    os.makedirs(log_dir, exist_ok=True)

    # Get current timestamp for log file naming
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    # Configure root logger
    root_logger = logging.getLogger()
    root_logger.setLevel(getattr(logging, log_level.upper()))

    # Clear existing handlers
    root_logger.handlers.clear()

    # Create formatters
    detailed_formatter = logging.Formatter(
        '%(asctime)s [%(levelname)8s] %(name)s:%(lineno)d: %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S'
    )

    simple_formatter = logging.Formatter(
        '%(asctime)s [%(levelname)8s] %(name)s: %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S'
    )

    # Console handler for immediate feedback
    console_handler = logging.StreamHandler()
    console_handler.setLevel(logging.INFO)
    console_handler.setFormatter(simple_formatter)
    root_logger.addHandler(console_handler)

    # Main log file handler
    main_log_file = os.path.join(log_dir, f"test_execution_{timestamp}.log")
    file_handler = logging.handlers.RotatingFileHandler(
        main_log_file,
        maxBytes=10*1024*1024,  # 10MB
        backupCount=5
    )
    file_handler.setLevel(logging.DEBUG)
    file_handler.setFormatter(detailed_formatter)
    root_logger.addHandler(file_handler)

    # Request/Response specific log file
    request_log_file = os.path.join(log_dir, f"soap_requests_{timestamp}.log")
    request_handler = logging.handlers.RotatingFileHandler(
        request_log_file,
        maxBytes=5*1024*1024,  # 5MB
        backupCount=3
    )
    request_handler.setLevel(logging.DEBUG)
    request_handler.setFormatter(detailed_formatter)

    # Create specialized loggers
    request_logger = logging.getLogger('onvif.requests')
    response_logger = logging.getLogger('onvif.responses')
    request_logger.addHandler(request_handler)
    response_logger.addHandler(request_handler)

    # Performance specific log file
    perf_log_file = os.path.join(log_dir, f"performance_{timestamp}.log")
    perf_handler = logging.handlers.RotatingFileHandler(
        perf_log_file,
        maxBytes=2*1024*1024,  # 2MB
        backupCount=3
    )
    perf_handler.setLevel(logging.INFO)
    perf_handler.setFormatter(simple_formatter)

    perf_logger = logging.getLogger('onvif.performance')
    perf_logger.addHandler(perf_handler)

    # Test execution specific log file
    test_log_file = os.path.join(log_dir, f"test_execution_{timestamp}.log")
    test_handler = logging.handlers.RotatingFileHandler(
        test_log_file,
        maxBytes=5*1024*1024,  # 5MB
        backupCount=3
    )
    test_handler.setLevel(logging.INFO)
    test_handler.setFormatter(detailed_formatter)

    test_logger = logging.getLogger('onvif.tests')
    test_logger.addHandler(test_handler)

    # Configure specific logger levels
    logging.getLogger('onvif.requests').setLevel(logging.DEBUG)
    logging.getLogger('onvif.responses').setLevel(logging.DEBUG)
    logging.getLogger('onvif.tests').setLevel(logging.INFO)
    logging.getLogger('onvif.performance').setLevel(logging.INFO)

    # Reduce noise from third-party libraries
    logging.getLogger('urllib3').setLevel(logging.WARNING)
    logging.getLogger('requests').setLevel(logging.WARNING)
    logging.getLogger('zeep').setLevel(logging.WARNING)

    # Log configuration completion
    root_logger.info("=== LOGGING CONFIGURATION COMPLETE ===")
    root_logger.info(f"Log level: {log_level}")
    root_logger.info(f"Log directory: {log_dir}")
    root_logger.info(f"Main log file: {main_log_file}")
    root_logger.info(f"Request log file: {request_log_file}")
    root_logger.info(f"Performance log file: {perf_log_file}")
    root_logger.info(f"Test execution log file: {test_log_file}")


def get_logger(name: str) -> logging.Logger:
    """
    Get a logger with the specified name

    Args:
        name: Logger name

    Returns:
        Logger instance
    """
    return logging.getLogger(name)


def log_test_session_start(session_name: str) -> None:
    """
    Log the start of a test session

    Args:
        session_name: Name of the test session
    """
    logger = get_logger('onvif.tests')
    logger.info("=" * 80)
    logger.info(f"TEST SESSION START: {session_name}")
    logger.info(f"Timestamp: {datetime.now().isoformat()}")
    logger.info("=" * 80)


def log_test_session_end(session_name: str, results: dict) -> None:
    """
    Log the end of a test session with results

    Args:
        session_name: Name of the test session
        results: Test results dictionary
    """
    logger = get_logger('onvif.tests')
    logger.info("=" * 80)
    logger.info(f"TEST SESSION END: {session_name}")
    logger.info(f"Timestamp: {datetime.now().isoformat()}")
    logger.info(f"Results: {results}")
    logger.info("=" * 80)
