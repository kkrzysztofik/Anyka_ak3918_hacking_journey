/**
 * @file errorHandler.ts
 * @brief Comprehensive error handling utilities
 * @author ONVIF Camera Web Interface
 * @date 2024
 */

/**
 * @brief Error severity levels
 */
export enum ErrorSeverity {
  LOW = 'low',
  MEDIUM = 'medium',
  HIGH = 'high',
  CRITICAL = 'critical'
}

/**
 * @brief Error categories
 */
export enum ErrorCategory {
  NETWORK = 'network',
  VALIDATION = 'validation',
  PARSING = 'parsing',
  AUTHENTICATION = 'authentication',
  AUTHORIZATION = 'authorization',
  CONFIGURATION = 'configuration',
  RUNTIME = 'runtime',
  UNKNOWN = 'unknown'
}

/**
 * @brief Standardized error interface
 */
export interface StandardError {
  code: string;
  message: string;
  severity: ErrorSeverity;
  category: ErrorCategory;
  timestamp: Date;
  context?: Record<string, any>;
  originalError?: Error;
}

/**
 * @brief Error handling utilities
 */
export class ErrorHandler {
  /**
   * @brief Create a standardized error from various error types
   * @param error Error object or string
   * @param category Error category
   * @param severity Error severity level
   * @param context Additional context information
   * @returns Standardized error object
   */
  static createStandardError(
    error: unknown,
    category: ErrorCategory = ErrorCategory.UNKNOWN,
    severity: ErrorSeverity = ErrorSeverity.MEDIUM,
    context?: Record<string, any>
  ): StandardError {
    const timestamp = new Date();
    
    let message: string;
    let code: string;
    let originalError: Error | undefined;

    if (error instanceof Error) {
      message = error.message;
      code = error.name || 'UNKNOWN_ERROR';
      originalError = error;
    } else if (typeof error === 'string') {
      message = error;
      code = 'STRING_ERROR';
    } else {
      message = 'An unknown error occurred';
      code = 'UNKNOWN_ERROR';
    }

    return {
      code,
      message,
      severity,
      category,
      timestamp,
      context,
      originalError
    };
  }

  /**
   * @brief Handle service errors with standardized format
   * @param error Error to handle
   * @param serviceName Name of the service where error occurred
   * @param operation Operation being performed when error occurred
   * @returns Standardized error response
   */
  static handleServiceError(
    error: unknown,
    serviceName: string,
    operation: string
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.RUNTIME,
      ErrorSeverity.MEDIUM,
      { serviceName, operation }
    );

    // Log error for debugging
    console.error(`Service error in ${serviceName}.${operation}:`, standardError);

    return {
      success: false,
      error: standardError.message,
      details: standardError
    };
  }

  /**
   * @brief Handle network errors
   * @param error Network error
   * @param url URL that failed
   * @param method HTTP method used
   * @returns Standardized error response
   */
  static handleNetworkError(
    error: unknown,
    url: string,
    method: string = 'GET'
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.NETWORK,
      ErrorSeverity.HIGH,
      { url, method }
    );

    // Enhance error message based on error type
    if (error instanceof Error) {
      if (error.message.includes('timeout')) {
        standardError.message = `Request timeout: ${url}`;
        standardError.severity = ErrorSeverity.MEDIUM;
      } else if (error.message.includes('Network Error')) {
        standardError.message = `Network error: Unable to reach ${url}`;
        standardError.severity = ErrorSeverity.HIGH;
      } else if (error.message.includes('404')) {
        standardError.message = `Not found: ${url}`;
        standardError.severity = ErrorSeverity.MEDIUM;
      } else if (error.message.includes('403')) {
        standardError.message = `Access forbidden: ${url}`;
        standardError.severity = ErrorSeverity.HIGH;
      } else if (error.message.includes('500')) {
        standardError.message = `Server error: ${url}`;
        standardError.severity = ErrorSeverity.HIGH;
      }
    }

    console.error('Network error:', standardError);

    return {
      success: false,
      error: standardError.message,
      details: standardError
    };
  }

  /**
   * @brief Handle validation errors
   * @param error Validation error
   * @param field Field that failed validation
   * @param value Value that failed validation
   * @returns Standardized error response
   */
  static handleValidationError(
    error: unknown,
    field: string,
    value: any
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.VALIDATION,
      ErrorSeverity.MEDIUM,
      { field, value }
    );

    console.warn('Validation error:', standardError);

    return {
      success: false,
      error: standardError.message,
      details: standardError
    };
  }

  /**
   * @brief Handle parsing errors
   * @param error Parsing error
   * @param dataType Type of data being parsed
   * @param data Data that failed to parse
   * @returns Standardized error response
   */
  static handleParsingError(
    error: unknown,
    dataType: string,
    data?: any
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.PARSING,
      ErrorSeverity.HIGH,
      { dataType, dataLength: data ? (typeof data === 'string' ? data.length : JSON.stringify(data).length) : 0 }
    );

    console.error('Parsing error:', standardError);

    return {
      success: false,
      error: standardError.message,
      details: standardError
    };
  }

  /**
   * @brief Handle configuration errors
   * @param error Configuration error
   * @param configKey Configuration key that failed
   * @param configValue Configuration value that failed
   * @returns Standardized error response
   */
  static handleConfigurationError(
    error: unknown,
    configKey: string,
    configValue?: any
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.CONFIGURATION,
      ErrorSeverity.HIGH,
      { configKey, configValue }
    );

    console.error('Configuration error:', standardError);

    return {
      success: false,
      error: standardError.message,
      details: standardError
    };
  }

  /**
   * @brief Handle authentication errors
   * @param error Authentication error
   * @param username Username that failed authentication
   * @returns Standardized error response
   */
  static handleAuthenticationError(
    error: unknown,
    username?: string
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.AUTHENTICATION,
      ErrorSeverity.HIGH,
      { username }
    );

    // Don't log sensitive information
    console.error('Authentication error:', {
      code: standardError.code,
      message: standardError.message,
      severity: standardError.severity,
      category: standardError.category,
      timestamp: standardError.timestamp
    });

    return {
      success: false,
      error: 'Authentication failed',
      details: standardError
    };
  }

  /**
   * @brief Handle authorization errors
   * @param error Authorization error
   * @param resource Resource that was denied access
   * @param action Action that was denied
   * @returns Standardized error response
   */
  static handleAuthorizationError(
    error: unknown,
    resource: string,
    action: string
  ): { success: false; error: string; details?: StandardError } {
    const standardError = this.createStandardError(
      error,
      ErrorCategory.AUTHORIZATION,
      ErrorSeverity.HIGH,
      { resource, action }
    );

    console.warn('Authorization error:', standardError);

    return {
      success: false,
      error: 'Access denied',
      details: standardError
    };
  }

  /**
   * @brief Get user-friendly error message
   * @param error Standardized error
   * @returns User-friendly error message
   */
  static getUserFriendlyMessage(error: StandardError): string {
    switch (error.category) {
      case ErrorCategory.NETWORK:
        return 'Network connection failed. Please check your internet connection and try again.';
      case ErrorCategory.VALIDATION:
        return 'Invalid input provided. Please check your input and try again.';
      case ErrorCategory.PARSING:
        return 'Failed to process data. Please try again or contact support.';
      case ErrorCategory.AUTHENTICATION:
        return 'Authentication failed. Please check your credentials.';
      case ErrorCategory.AUTHORIZATION:
        return 'Access denied. You do not have permission to perform this action.';
      case ErrorCategory.CONFIGURATION:
        return 'Configuration error. Please check your settings.';
      case ErrorCategory.RUNTIME:
        return 'An unexpected error occurred. Please try again.';
      default:
        return 'An unknown error occurred. Please try again.';
    }
  }

  /**
   * @brief Check if error is retryable
   * @param error Standardized error
   * @returns True if error is retryable
   */
  static isRetryableError(error: StandardError): boolean {
    switch (error.category) {
      case ErrorCategory.NETWORK:
        return error.severity !== ErrorSeverity.CRITICAL;
      case ErrorCategory.PARSING:
        return false; // Parsing errors are usually not retryable
      case ErrorCategory.VALIDATION:
        return false; // Validation errors are usually not retryable
      case ErrorCategory.AUTHENTICATION:
        return false; // Authentication errors are usually not retryable
      case ErrorCategory.AUTHORIZATION:
        return false; // Authorization errors are usually not retryable
      case ErrorCategory.CONFIGURATION:
        return false; // Configuration errors are usually not retryable
      case ErrorCategory.RUNTIME:
        return error.severity === ErrorSeverity.LOW || error.severity === ErrorSeverity.MEDIUM;
      default:
        return false;
    }
  }

  /**
   * @brief Get retry delay based on error severity
   * @param error Standardized error
   * @param attempt Current retry attempt (0-based)
   * @returns Delay in milliseconds
   */
  static getRetryDelay(error: StandardError, attempt: number): number {
    if (!this.isRetryableError(error)) {
      return 0;
    }

    const baseDelay = error.severity === ErrorSeverity.HIGH ? 5000 : 1000;
    const maxDelay = 30000; // 30 seconds max
    const delay = Math.min(baseDelay * Math.pow(2, attempt), maxDelay);
    
    // Add jitter to prevent thundering herd
    const jitter = Math.random() * 1000;
    return delay + jitter;
  }

  /**
   * @brief Log error with appropriate level
   * @param error Standardized error
   * @param additionalContext Additional context to log
   */
  static logError(error: StandardError, additionalContext?: Record<string, any>): void {
    const logData = {
      ...error,
      context: { ...error.context, ...additionalContext }
    };

    switch (error.severity) {
      case ErrorSeverity.CRITICAL:
        console.error('CRITICAL ERROR:', logData);
        break;
      case ErrorSeverity.HIGH:
        console.error('HIGH SEVERITY ERROR:', logData);
        break;
      case ErrorSeverity.MEDIUM:
        console.warn('MEDIUM SEVERITY ERROR:', logData);
        break;
      case ErrorSeverity.LOW:
        console.info('LOW SEVERITY ERROR:', logData);
        break;
    }
  }
}

/**
 * @brief Factory function to create error handler
 * @returns ErrorHandler instance
 */
export const createErrorHandler = (): typeof ErrorHandler => {
  return ErrorHandler;
};
