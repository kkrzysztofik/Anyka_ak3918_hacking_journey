/**
 * @file xmlSanitizer.ts
 * @brief XML sanitization using DOMPurify library
 * @author ONVIF Camera Web Interface
 * @date 2024
 */

import DOMPurify from 'dompurify';

/**
 * @brief XML sanitization configuration
 */
interface XMLSanitizerConfig {
  allowXmlDeclarations?: boolean;
  allowNamespaces?: boolean;
  allowProcessingInstructions?: boolean;
  maxSize?: number;
  allowedTags?: string[];
  allowedAttributes?: Record<string, string[]>;
}

/**
 * @brief Default configuration for XML sanitization
 */
const DEFAULT_CONFIG: XMLSanitizerConfig = {
  allowXmlDeclarations: true,
  allowNamespaces: true,
  allowProcessingInstructions: false,
  maxSize: 10 * 1024 * 1024, // 10MB
  allowedTags: [
    // ONVIF specific tags
    'soap:Envelope', 'soap:Body', 'soap:Header',
    'tds:GetDeviceInformation', 'tds:GetCapabilities',
    'tptz:Move', 'tptz:Stop', 'timg:SetImagingSettings',
    // Common XML elements
    'root', 'element', 'value', 'text', 'data'
  ],
  allowedAttributes: {
    'soap:Envelope': ['xmlns:soap', 'xmlns:tds', 'xmlns:tptz', 'xmlns:timg'],
    'soap:Body': [],
    'soap:Header': [],
    '*': ['xmlns', 'xmlns:*']
  }
};

/**
 * @brief XML sanitization utility class
 */
export class XMLSanitizer {
  private config: XMLSanitizerConfig;

  constructor(config: Partial<XMLSanitizerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * @brief Sanitize XML input using DOMPurify
   * @param xmlData XML string to sanitize
   * @returns Sanitized XML string
   * @throws Error if XML is too large or invalid
   */
  sanitize(xmlData: string): string {
    if (!xmlData || typeof xmlData !== 'string') {
      throw new Error('Invalid XML data: expected non-empty string');
    }

    // Check size limit
    if (xmlData.length > this.config.maxSize!) {
      throw new Error(`XML data too large: ${xmlData.length} bytes (max: ${this.config.maxSize})`);
    }

    // Configure DOMPurify for XML
    const purifyConfig = {
      ALLOWED_TAGS: this.config.allowedTags,
      ALLOWED_ATTR: this.getAllowedAttributes(),
      ALLOW_DATA_ATTR: false,
      ALLOW_UNKNOWN_PROTOCOLS: false,
      SANITIZE_DOM: true,
      KEEP_CONTENT: true,
      RETURN_DOM: false,
      RETURN_DOM_FRAGMENT: false,
      RETURN_DOM_IMPORT: false,
      // XML specific settings
      SAFE_FOR_XML: true,
      ALLOWED_URI_REGEXP: /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|cid|xmpp):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i
    };

    try {
      // Sanitize the XML
      const sanitized = DOMPurify.sanitize(xmlData, purifyConfig);
      
      // Post-process for XML-specific requirements
      return this.postProcessXML(sanitized);
    } catch (error) {
      throw new Error(`XML sanitization failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  /**
   * @brief Get allowed attributes for DOMPurify configuration
   * @returns Array of allowed attributes
   */
  private getAllowedAttributes(): string[] {
    const attributes = new Set<string>();
    
    Object.values(this.config.allowedAttributes || {}).forEach(attrList => {
      attrList.forEach(attr => attributes.add(attr));
    });

    return Array.from(attributes);
  }

  /**
   * @brief Post-process sanitized XML to ensure XML validity
   * @param xml Sanitized XML string
   * @returns Post-processed XML string
   */
  private postProcessXML(xml: string): string {
    let processed = xml;

    // Ensure XML declaration is preserved if allowed
    if (this.config.allowXmlDeclarations && !processed.includes('<?xml')) {
      processed = '<?xml version="1.0" encoding="UTF-8"?>\n' + processed;
    }

    // Remove any remaining dangerous content
    processed = processed
      .replace(/javascript:/gi, '')
      .replace(/on\w+\s*=/gi, '')
      .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '');

    return processed;
  }

  /**
   * @brief Validate XML structure after sanitization
   * @param xml Sanitized XML string
   * @returns True if valid, false otherwise
   */
  validate(xml: string): boolean {
    try {
      // Basic XML structure validation
      const hasRoot = /<[^\/][^>]*>/.test(xml);
      const balancedTags = this.checkBalancedTags(xml);
      
      return hasRoot && balancedTags;
    } catch {
      return false;
    }
  }

  /**
   * @brief Check if XML tags are properly balanced
   * @param xml XML string to check
   * @returns True if balanced, false otherwise
   */
  private checkBalancedTags(xml: string): boolean {
    const tagStack: string[] = [];
    const tagRegex = /<\/?([a-zA-Z][a-zA-Z0-9:]*)[^>]*>/g;
    let match;

    while ((match = tagRegex.exec(xml)) !== null) {
      const fullTag = match[0];
      const tagName = match[1];
      
      if (fullTag.startsWith('</')) {
        // Closing tag
        if (tagStack.length === 0 || tagStack.pop() !== tagName) {
          return false;
        }
      } else if (!fullTag.endsWith('/>')) {
        // Opening tag (not self-closing)
        tagStack.push(tagName);
      }
    }

    return tagStack.length === 0;
  }

  /**
   * @brief Update sanitizer configuration
   * @param newConfig New configuration options
   */
  updateConfig(newConfig: Partial<XMLSanitizerConfig>): void {
    this.config = { ...this.config, ...newConfig };
  }

  /**
   * @brief Get current configuration
   * @returns Current sanitizer configuration
   */
  getConfig(): XMLSanitizerConfig {
    return { ...this.config };
  }
}

/**
 * @brief Factory function to create XML sanitizer
 * @param config Optional configuration
 * @returns XMLSanitizer instance
 */
export const createXMLSanitizer = (config?: Partial<XMLSanitizerConfig>): XMLSanitizer => {
  return new XMLSanitizer(config);
};

/**
 * @brief Default XML sanitizer instance
 */
export const defaultXMLSanitizer = new XMLSanitizer();
