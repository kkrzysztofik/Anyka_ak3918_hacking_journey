import { XMLParser, XMLBuilder } from 'fast-xml-parser';
import { createXMLSanitizer } from './xmlSanitizer';

/**
 * @file xmlParser.ts
 * @brief Centralized XML parsing and building utilities for ONVIF SOAP communication
 * @author ONVIF Camera Web Interface
 * @date 2024
 */

/**
 * @brief Configuration for XML parsing
 */
const XML_PARSER_CONFIG = {
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  textNodeName: '#text',
  parseAttributeValue: true,
  trimValues: true,
  parseNodeValue: true,
  parseTrueNumberOnly: false,
  arrayMode: false
};

/**
 * @brief Configuration for XML building
 */
const XML_BUILDER_CONFIG = {
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  textNodeName: '#text',
  format: true,
  indentBy: '  ',
  suppressEmptyNode: true
};

/**
 * @brief Centralized XML parsing and building service
 * @description Provides consistent XML parsing and building functionality across the application
 */
export class XMLParserService {
  private static parser: XMLParser;
  private static builder: XMLBuilder;
  private static sanitizer = createXMLSanitizer();

  /**
   * @brief Initialize the XML parser and builder instances
   * @description Creates singleton instances of XMLParser and XMLBuilder with consistent configuration
   */
  private static initialize(): void {
    if (!this.parser) {
      this.parser = new XMLParser(XML_PARSER_CONFIG);
    }
    if (!this.builder) {
      this.builder = new XMLBuilder(XML_BUILDER_CONFIG);
    }
  }

  /**
   * @brief Sanitize XML input using DOMPurify-based sanitizer
   * @param xmlData XML string to sanitize
   * @returns Sanitized XML string
   * @note Uses professional-grade sanitization library for better security
   */
  private static sanitizeXmlInput(xmlData: string): string {
    if (!xmlData || typeof xmlData !== 'string') {
      return '';
    }

    // Use the professional XML sanitizer
    return this.sanitizer.sanitize(xmlData);
  }

  /**
   * @brief Parse XML string into JavaScript object
   * @param xmlData XML string to parse
   * @returns Parsed JavaScript object
   * @throws Error if XML parsing fails
   * @note Logs parsing errors for debugging and sanitizes input for security
   */
  static parse(xmlData: string): any {
    this.initialize();
    
    if (!xmlData || typeof xmlData !== 'string') {
      const error = new Error('Invalid XML data: expected non-empty string');
      console.error('XMLParserService.parse failed:', error.message);
      throw error;
    }

    try {
      // Sanitize input to prevent XSS attacks
      const sanitizedData = this.sanitizeXmlInput(xmlData);
      
      const result = this.parser.parse(sanitizedData);
      console.debug('XML parsing successful, parsed data keys:', Object.keys(result || {}));
      return result;
    } catch (error) {
      const errorMessage = `XML parsing failed: ${error instanceof Error ? error.message : 'Unknown error'}`;
      console.error('XMLParserService.parse failed:', errorMessage, {
        xmlDataLength: xmlData.length,
        xmlDataPreview: xmlData.substring(0, 200) + (xmlData.length > 200 ? '...' : '')
      });
      throw new Error(errorMessage);
    }
  }

  /**
   * @brief Sanitize data before XML building to prevent injection attacks
   * @param data JavaScript object to sanitize
   * @returns Sanitized object safe for XML building
   */
  private static sanitizeDataForBuilding(data: any): any {
    if (!data || typeof data !== 'object') {
      return data;
    }

    const sanitizeValue = (value: any): any => {
      if (typeof value === 'string') {
        // Use XML sanitizer for string values
        try {
          return this.sanitizer.sanitize(value);
        } catch (error) {
          // Fallback to basic escaping if sanitizer fails
          console.warn('String sanitization failed, using basic escaping:', error);
          return value
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#39;');
        }
      } else if (Array.isArray(value)) {
        return value.map(sanitizeValue);
      } else if (value && typeof value === 'object') {
        const sanitized: any = {};
        for (const [key, val] of Object.entries(value)) {
          // Sanitize object keys
          const sanitizedKey = key.replace(/[^a-zA-Z0-9_:-]/g, '');
          sanitized[sanitizedKey] = sanitizeValue(val);
        }
        return sanitized;
      }
      return value;
    };

    return sanitizeValue(data);
  }

  /**
   * @brief Build JavaScript object into XML string
   * @param data JavaScript object to convert to XML
   * @returns XML string
   * @throws Error if XML building fails
   * @note Logs building errors for debugging and sanitizes data for security
   */
  static build(data: any): string {
    this.initialize();
    
    if (!data || typeof data !== 'object') {
      const error = new Error('Invalid data for XML building: expected object');
      console.error('XMLParserService.build failed:', error.message);
      throw error;
    }

    try {
      // Sanitize data before building to prevent injection attacks
      const sanitizedData = this.sanitizeDataForBuilding(data);
      
      const result = this.builder.build(sanitizedData);
      
      // Additional security: limit output size
      const MAX_OUTPUT_SIZE = 10 * 1024 * 1024; // 10MB limit
      if (result.length > MAX_OUTPUT_SIZE) {
        throw new Error(`XML output too large: ${result.length} bytes (max: ${MAX_OUTPUT_SIZE})`);
      }
      
      console.debug('XML building successful, output length:', result.length);
      return result;
    } catch (error) {
      const errorMessage = `XML building failed: ${error instanceof Error ? error.message : 'Unknown error'}`;
      console.error('XMLParserService.build failed:', errorMessage, {
        dataKeys: Object.keys(data || {}),
        dataType: typeof data
      });
      throw new Error(errorMessage);
    }
  }

  /**
   * @brief Extract value from parsed XML using dot notation path
   * @param data Parsed XML object
   * @param path Dot notation path to the desired value (e.g., 'soap:Envelope.soap:Body.tds:Manufacturer')
   * @returns Extracted value or null if not found
   * @note Logs extraction attempts for debugging
   */
  static extractValue(data: any, path: string): any {
    if (!data || !path) {
      console.warn('XMLParserService.extractValue: Invalid parameters', { data: !!data, path });
      return null;
    }

    const keys = path.split('.');
    let current = data;
    
    try {
      for (const key of keys) {
        if (current && typeof current === 'object') {
          current = current[key];
        } else {
          console.debug(`XMLParserService.extractValue: Path not found at key '${key}' in path '${path}'`);
          return null;
        }
      }
      
      console.debug(`XMLParserService.extractValue: Successfully extracted value for path '${path}'`, { value: current });
      return current;
    } catch (error) {
      console.error(`XMLParserService.extractValue: Error extracting value for path '${path}'`, error);
      return null;
    }
  }

  /**
   * @brief Find value in nested XML object by tag name
   * @param data Parsed XML object
   * @param tagName Tag name to search for
   * @returns Found value or null if not found
   * @note Recursively searches through nested objects
   */
  static findValueByTag(data: any, tagName: string): string | null {
    if (!data || !tagName) {
      console.warn('XMLParserService.findValueByTag: Invalid parameters', { data: !!data, tagName });
      return null;
    }

    const findValue = (obj: any, tag: string): string | null => {
      if (typeof obj !== 'object' || obj === null) {
        return null;
      }

      if (obj[tag] !== undefined) {
        const value = typeof obj[tag] === 'string' ? obj[tag] : String(obj[tag]);
        console.debug(`XMLParserService.findValueByTag: Found value for tag '${tag}'`, { value });
        return value;
      }

      // Search in nested objects
      for (const key in obj) {
        if (obj.hasOwnProperty(key)) {
          const result = findValue(obj[key], tag);
          if (result !== null) {
            return result;
          }
        }
      }

      return null;
    };

    const result = findValue(data, tagName);
    if (result === null) {
      console.debug(`XMLParserService.findValueByTag: Tag '${tagName}' not found in XML data`);
    }
    return result;
  }

  /**
   * @brief Validate SOAP envelope structure
   * @param data Parsed XML object
   * @returns True if valid SOAP envelope, false otherwise
   * @note Logs validation results for debugging
   */
  static validateSOAPResponse(data: any): boolean {
    if (!data || typeof data !== 'object') {
      console.warn('XMLParserService.validateSOAPResponse: Invalid data type');
      return false;
    }

    const isValid = !!(data['soap:Envelope'] && 
                      data['soap:Envelope']['soap:Body']);
    
    if (!isValid) {
      console.warn('XMLParserService.validateSOAPResponse: Invalid SOAP envelope structure', {
        hasEnvelope: !!data['soap:Envelope'],
        hasBody: !!(data['soap:Envelope'] && data['soap:Envelope']['soap:Body']),
        dataKeys: Object.keys(data || {})
      });
    } else {
      console.debug('XMLParserService.validateSOAPResponse: Valid SOAP envelope');
    }
    
    return isValid;
  }

  /**
   * @brief Create SOAP envelope for ONVIF requests
   * @param body SOAP body content
   * @param namespaces Additional namespaces to include
   * @returns SOAP envelope object
   * @note Includes standard ONVIF namespaces
   */
  static createSOAPEnvelope(body: any, namespaces: Record<string, string> = {}): any {
    const defaultNamespaces = {
      '@_xmlns:soap': 'http://www.w3.org/2003/05/soap-envelope',
      '@_xmlns:tds': 'http://www.onvif.org/ver10/device/wsdl',
      '@_xmlns:tptz': 'http://www.onvif.org/ver20/ptz/wsdl',
      '@_xmlns:timg': 'http://www.onvif.org/ver20/imaging/wsdl',
      '@_xmlns:trt': 'http://www.onvif.org/ver10/media/wsdl',
      '@_xmlns:tt': 'http://www.onvif.org/ver10/schema'
    };

    const allNamespaces = { ...defaultNamespaces, ...namespaces };

    return {
      'soap:Envelope': {
        ...allNamespaces,
        'soap:Header': {},
        'soap:Body': body
      }
    };
  }
}

/**
 * @brief Factory function to create XML parser service instance
 * @returns XMLParserService instance
 */
export const createXMLParserService = (): XMLParserService => {
  return XMLParserService;
};
