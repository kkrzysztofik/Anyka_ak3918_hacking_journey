/**
 * @file constants.h
 * @brief Central SOAP/XML template constants and configuration paths.
 */
#ifndef ONVIF_CONSTANTS_H
#define ONVIF_CONSTANTS_H

#define ONVIF_CONFIG_FILE "/etc/jffs2/ankya_cfg.ini"

/* WS-Discovery (kept original names) */
#define WSD_HELLO_TEMPLATE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?>" \
"<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:wsa=\"http://schemas.xmlsoap.org/ws/2004/08/addressing\" xmlns:wsd=\"http://schemas.xmlsoap.org/ws/2005/04/discovery\"><soap:Header><wsa:MessageID>urn:uuid:%s</wsa:MessageID><wsa:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To><wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Hello</wsa:Action></soap:Header><soap:Body><wsd:Hello><wsa:EndpointReference><wsa:Address>%s</wsa:Address></wsa:EndpointReference><wsd:Types>tds:Device</wsd:Types><wsd:Scopes>onvif://www.onvif.org/name/Anyka onvif://www.onvif.org/type/NetworkVideoTransmitter</wsd:Scopes><wsd:XAddrs>http://%s:%d/onvif/device_service</wsd:XAddrs><wsd:MetadataVersion>1</wsd:MetadataVersion></wsd:Hello></soap:Body></soap:Envelope>"

#define WSD_BYE_TEMPLATE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?>" \
"<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:wsa=\"http://schemas.xmlsoap.org/ws/2004/08/addressing\" xmlns:wsd=\"http://schemas.xmlsoap.org/ws/2005/04/discovery\"><soap:Header><wsa:MessageID>urn:uuid:%s</wsa:MessageID><wsa:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To><wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Bye</wsa:Action></soap:Header><soap:Body><wsd:Bye><wsa:EndpointReference><wsa:Address>%s</wsa:Address></wsa:EndpointReference></wsd:Bye></soap:Body></soap:Envelope>"

#define WSD_PROBE_MATCH_TEMPLATE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?>" \
"<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:wsa=\"http://schemas.xmlsoap.org/ws/2004/08/addressing\" xmlns:wsd=\"http://schemas.xmlsoap.org/ws/2005/04/discovery\" xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"><soap:Header><wsa:MessageID>urn:uuid:%s</wsa:MessageID><wsa:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To><wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</wsa:Action></soap:Header><soap:Body><wsd:ProbeMatches><wsd:ProbeMatch><wsa:EndpointReference><wsa:Address>%s</wsa:Address></wsa:EndpointReference><wsd:Types>tds:Device</wsd:Types><wsd:Scopes>onvif://www.onvif.org/name/Anyka onvif://www.onvif.org/type/NetworkVideoTransmitter</wsd:Scopes><wsd:XAddrs>http://%s:%d/onvif/device_service</wsd:XAddrs><wsd:MetadataVersion>1</wsd:MetadataVersion></wsd:ProbeMatch></wsd:ProbeMatches></soap:Body></soap:Envelope>"

/* Device */
#define ONVIF_SOAP_DEVICE_GET_DEVICE_INFORMATION_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"><s:Body><tds:GetDeviceInformationResponse><tds:Manufacturer>%s</tds:Manufacturer><tds:Model>%s</tds:Model><tds:FirmwareVersion>%s</tds:FirmwareVersion><tds:SerialNumber>%s</tds:SerialNumber><tds:HardwareId>%s</tds:HardwareId></tds:GetDeviceInformationResponse></s:Body></s:Envelope>"

/* Imaging */
#define ONVIF_SOAP_IMAGING_GET_SETTINGS_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><trt:GetImagingSettingsResponse><trt:ImagingSettings><tt:Brightness>%d</tt:Brightness><tt:Contrast>%d</tt:Contrast><tt:Saturation>%d</tt:Saturation><tt:Sharpness>%d</tt:Sharpness><tt:ColorSaturation>%d</tt:ColorSaturation></trt:ImagingSettings></trt:GetImagingSettingsResponse></s:Body></s:Envelope>"
#define ONVIF_SOAP_IMAGING_SET_SETTINGS_OK \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"><s:Body><trt:SetImagingSettingsResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_IMAGING_SET_SETTINGS_FAIL \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\"><s:Body><s:Fault><s:Code><s:Value>s:Receiver</s:Value></s:Code><s:Reason><s:Text>Failed to set imaging settings</s:Text></s:Reason></s:Fault></s:Body></s:Envelope>"
#define ONVIF_SOAP_IMAGING_GET_OPTIONS_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><trt:GetImagingOptionsResponse><trt:ImagingOptions><tt:Brightness><tt:Min>-100</tt:Min><tt:Max>100</tt:Max></tt:Brightness><tt:Contrast><tt:Min>-100</tt:Min><tt:Max>100</tt:Max></tt:Contrast><tt:Saturation><tt:Min>-100</tt:Min><tt:Max>100</tt:Max></tt:Saturation><tt:Sharpness><tt:Min>-100</tt:Min><tt:Max>100</tt:Max></tt:Sharpness><tt:ColorSaturation><tt:Min>-180</tt:Min><tt:Max>180</tt:Max></tt:ColorSaturation></trt:ImagingOptions></trt:GetImagingOptionsResponse></s:Body></s:Envelope>"

/* Media */
#define ONVIF_SOAP_MEDIA_GET_PROFILES_HEADER \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><trt:GetProfilesResponse>"
#define ONVIF_SOAP_MEDIA_GET_PROFILES_PROFILE_ENTRY \
"<trt:Profiles token=\"%s\" fixed=\"true\"><trt:Name>%s</trt:Name><trt:VideoSourceConfiguration><tt:Name>VideoSourceConfig</tt:Name><tt:UseCount>%d</tt:UseCount><tt:SourceToken>%s</tt:SourceToken><tt:Bounds width=\"%d\" height=\"%d\" x=\"0\" y=\"0\"/></trt:VideoSourceConfiguration><trt:VideoEncoderConfiguration><tt:Name>%s</tt:Name><tt:UseCount>1</tt:UseCount><tt:Encoding>H264</tt:Encoding><tt:Resolution><tt:Width>%d</tt:Width><tt:Height>%d</tt:Height></tt:Resolution><tt:Quality>%d</tt:Quality><tt:RateControl><tt:FrameRateLimit>%d</tt:FrameRateLimit><tt:EncodingInterval>%d</tt:EncodingInterval><tt:BitrateLimit>%d</tt:BitrateLimit></tt:RateControl><tt:H264><tt:GovLength>%d</tt:GovLength><tt:H264Profile>Main</tt:H264Profile></tt:H264></trt:VideoEncoderConfiguration><trt:AudioSourceConfiguration><tt:Name>AudioSourceConfig</tt:Name><tt:UseCount>1</tt:UseCount><tt:SourceToken>%s</tt:SourceToken></trt:AudioSourceConfiguration><trt:AudioEncoderConfiguration><tt:Name>AAC Encoder</tt:Name><tt:UseCount>1</tt:UseCount><tt:Encoding>AAC</tt:Encoding><tt:Bitrate>%d</tt:Bitrate><tt:SampleRate>%d</tt:SampleRate></trt:AudioEncoderConfiguration></trt:Profiles>"
#define ONVIF_SOAP_MEDIA_GET_PROFILES_FOOTER "</trt:GetProfilesResponse></s:Body></s:Envelope>"
#define ONVIF_SOAP_MEDIA_GET_STREAM_URI_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><trt:GetStreamUriResponse><trt:MediaUri><tt:Uri>%s</tt:Uri><tt:InvalidAfterConnect>false</tt:InvalidAfterConnect><tt:InvalidAfterReboot>false</tt:InvalidAfterReboot><tt:Timeout>PT%dS</tt:Timeout></trt:MediaUri></trt:GetStreamUriResponse></s:Body></s:Envelope>"
#define ONVIF_SOAP_MEDIA_GET_SNAPSHOT_URI_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><trt:GetSnapshotUriResponse><trt:MediaUri><tt:Uri>%s</tt:Uri><tt:InvalidAfterConnect>false</tt:InvalidAfterConnect><tt:InvalidAfterReboot>false</tt:InvalidAfterReboot><tt:Timeout>PT%dS</tt:Timeout></trt:MediaUri></trt:GetSnapshotUriResponse></s:Body></s:Envelope>"

#define ONVIF_SOAP_FAULT_GENERIC \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\"><s:Body><s:Fault><s:Code><s:Value>s:Receiver</s:Value></s:Code><s:Reason><s:Text>%s</s:Text></s:Reason></s:Fault></s:Body></s:Envelope>"

/* PTZ */
#define ONVIF_SOAP_PTZ_GET_STATUS_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><tptz:GetStatusResponse><tptz:PTZStatus><tt:Position><tt:PanTilt x=\"%f\" y=\"%f\"/><tt:Zoom x=\"%f\"/></tt:Position><tt:MoveStatus><tt:PanTilt>%s</tt:PanTilt><tt:Zoom>IDLE</tt:Zoom></tt:MoveStatus><tt:UtcTime>%s</tt:UtcTime></tptz:PTZStatus></tptz:GetStatusResponse></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_ABSOLUTE_MOVE_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:AbsoluteMoveResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_RELATIVE_MOVE_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:RelativeMoveResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_CONTINUOUS_MOVE_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:ContinuousMoveResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_STOP_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:StopResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_GOTO_HOME_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:GotoHomePositionResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_SET_HOME_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:SetHomePositionResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_GOTO_PRESET_OK "<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:GotoPresetResponse/></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_SET_PRESET_RESPONSE \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"><s:Body><tptz:SetPresetResponse><tptz:PresetToken>%s</tptz:PresetToken></tptz:SetPresetResponse></s:Body></s:Envelope>"
#define ONVIF_SOAP_PTZ_GET_PRESETS_HEADER \
"<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body><tptz:GetPresetsResponse>"
#define ONVIF_SOAP_PTZ_GET_PRESETS_ENTRY \
"<tptz:Preset><tt:Name>%s</tt:Name><tt:Token>%s</tt:Token></tptz:Preset>"
#define ONVIF_SOAP_PTZ_GET_PRESETS_FOOTER "</tptz:GetPresetsResponse></s:Body></s:Envelope>"

#endif /* ONVIF_CONSTANTS_H */