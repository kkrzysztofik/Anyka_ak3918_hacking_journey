/**
 * @file soap_test_envelopes.h
 * @brief SOAP envelope test data for ONVIF gSOAP parsing unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SOAP_TEST_ENVELOPES_H
#define SOAP_TEST_ENVELOPES_H

/* SOAP 1.2 envelope header with ONVIF namespaces */
#define SOAP_ENVELOPE_HEADER \
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" \
    "<s:Envelope " \
    "xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" " \
    "xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\" " \
    "xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" " \
    "xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\" " \
    "xmlns:timg=\"http://www.onvif.org/ver20/imaging/wsdl\" " \
    "xmlns:tt=\"http://www.onvif.org/ver10/schema\">" \
    "<s:Body>"

/* SOAP 1.2 envelope footer */
#define SOAP_ENVELOPE_FOOTER \
    "</s:Body>" \
    "</s:Envelope>"

/* ============================================================================
 * Media Service Test Envelopes (6 operations)
 * ============================================================================ */

/* GetProfiles - Returns all media profiles */
#define SOAP_MEDIA_GET_PROFILES \
    SOAP_ENVELOPE_HEADER \
    "<trt:GetProfiles/>" \
    SOAP_ENVELOPE_FOOTER

/* GetStreamUri - Returns RTSP stream URI for a profile */
#define SOAP_MEDIA_GET_STREAM_URI \
    SOAP_ENVELOPE_HEADER \
    "<trt:GetStreamUri>" \
    "<trt:ProfileToken>profile_1</trt:ProfileToken>" \
    "<trt:StreamSetup>" \
    "<tt:Stream>RTP-Unicast</tt:Stream>" \
    "<tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport>" \
    "</trt:StreamSetup>" \
    "</trt:GetStreamUri>" \
    SOAP_ENVELOPE_FOOTER

/* CreateProfile - Creates a new media profile */
#define SOAP_MEDIA_CREATE_PROFILE \
    SOAP_ENVELOPE_HEADER \
    "<trt:CreateProfile>" \
    "<trt:Name>TestProfile</trt:Name>" \
    "<trt:Token>test_profile_token</trt:Token>" \
    "</trt:CreateProfile>" \
    SOAP_ENVELOPE_FOOTER

/* DeleteProfile - Deletes a media profile */
#define SOAP_MEDIA_DELETE_PROFILE \
    SOAP_ENVELOPE_HEADER \
    "<trt:DeleteProfile>" \
    "<trt:ProfileToken>profile_to_delete</trt:ProfileToken>" \
    "</trt:DeleteProfile>" \
    SOAP_ENVELOPE_FOOTER

/* SetVideoSourceConfiguration - Sets video source configuration */
#define SOAP_MEDIA_SET_VIDEO_SOURCE_CONFIG \
    SOAP_ENVELOPE_HEADER \
    "<trt:SetVideoSourceConfiguration>" \
    "<trt:Configuration token=\"video_src_config_1\">" \
    "<tt:Name>VideoSourceConfig</tt:Name>" \
    "<tt:SourceToken>video_source_0</tt:SourceToken>" \
    "<tt:Bounds x=\"0\" y=\"0\" width=\"1920\" height=\"1080\"/>" \
    "</trt:Configuration>" \
    "<trt:ForcePersistence>true</trt:ForcePersistence>" \
    "</trt:SetVideoSourceConfiguration>" \
    SOAP_ENVELOPE_FOOTER

/* SetVideoEncoderConfiguration - Sets video encoder configuration */
#define SOAP_MEDIA_SET_VIDEO_ENCODER_CONFIG \
    SOAP_ENVELOPE_HEADER \
    "<trt:SetVideoEncoderConfiguration>" \
    "<trt:Configuration token=\"video_enc_config_1\">" \
    "<tt:Name>VideoEncoderConfig</tt:Name>" \
    "<tt:Encoding>H264</tt:Encoding>" \
    "<tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>" \
    "<tt:Quality>4</tt:Quality>" \
    "<tt:RateControl><tt:FrameRateLimit>30</tt:FrameRateLimit><tt:BitrateLimit>4096</tt:BitrateLimit></tt:RateControl>" \
    "</trt:Configuration>" \
    "<trt:ForcePersistence>true</trt:ForcePersistence>" \
    "</trt:SetVideoEncoderConfiguration>" \
    SOAP_ENVELOPE_FOOTER

/* GetMetadataConfigurations - Returns metadata configurations */
#define SOAP_MEDIA_GET_METADATA_CONFIGURATIONS \
    SOAP_ENVELOPE_HEADER \
    "<trt:GetMetadataConfigurations/>" \
    SOAP_ENVELOPE_FOOTER

/* SetMetadataConfiguration - Sets metadata configuration */
#define SOAP_MEDIA_SET_METADATA_CONFIGURATION \
    SOAP_ENVELOPE_HEADER \
    "<trt:SetMetadataConfiguration>" \
    "<trt:Configuration token=\"MetadataConfig0\">" \
    "<tt:Name>Metadata Configuration</tt:Name>" \
    "<tt:SessionTimeout>60</tt:SessionTimeout>" \
    "<tt:Analytics>true</tt:Analytics>" \
    "</trt:Configuration>" \
    "<trt:ForcePersistence>true</trt:ForcePersistence>" \
    "</trt:SetMetadataConfiguration>" \
    SOAP_ENVELOPE_FOOTER

/* StartMulticastStreaming - Starts multicast streaming */
#define SOAP_MEDIA_START_MULTICAST_STREAMING \
    SOAP_ENVELOPE_HEADER \
    "<trt:StartMulticastStreaming>" \
    "<trt:ProfileToken>profile_1</trt:ProfileToken>" \
    "</trt:StartMulticastStreaming>" \
    SOAP_ENVELOPE_FOOTER

/* StopMulticastStreaming - Stops multicast streaming */
#define SOAP_MEDIA_STOP_MULTICAST_STREAMING \
    SOAP_ENVELOPE_HEADER \
    "<trt:StopMulticastStreaming>" \
    "<trt:ProfileToken>profile_1</trt:ProfileToken>" \
    "</trt:StopMulticastStreaming>" \
    SOAP_ENVELOPE_FOOTER

/* ============================================================================
 * PTZ Service Test Envelopes (6 operations)
 * ============================================================================ */

/* GetNodes - Returns PTZ nodes */
#define SOAP_PTZ_GET_NODES \
    SOAP_ENVELOPE_HEADER \
    "<tptz:GetNodes/>" \
    SOAP_ENVELOPE_FOOTER

/* AbsoluteMove - Moves PTZ to absolute position */
#define SOAP_PTZ_ABSOLUTE_MOVE \
    SOAP_ENVELOPE_HEADER \
    "<tptz:AbsoluteMove>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "<tptz:Position>" \
    "<tt:PanTilt x=\"0.5\" y=\"0.3\" space=\"http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace\"/>" \
    "<tt:Zoom x=\"0.0\" space=\"http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace\"/>" \
    "</tptz:Position>" \
    "<tptz:Speed>" \
    "<tt:PanTilt x=\"0.5\" y=\"0.5\" space=\"http://www.onvif.org/ver10/tptz/PanTiltSpaces/GenericSpeedSpace\"/>" \
    "<tt:Zoom x=\"0.5\" space=\"http://www.onvif.org/ver10/tptz/ZoomSpaces/ZoomGenericSpeedSpace\"/>" \
    "</tptz:Speed>" \
    "</tptz:AbsoluteMove>" \
    SOAP_ENVELOPE_FOOTER

/* AbsoluteMove without speed (optional field test) */
#define SOAP_PTZ_ABSOLUTE_MOVE_NO_SPEED \
    SOAP_ENVELOPE_HEADER \
    "<tptz:AbsoluteMove>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "<tptz:Position>" \
    "<tt:PanTilt x=\"0.5\" y=\"0.3\" space=\"http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace\"/>" \
    "<tt:Zoom x=\"0.0\" space=\"http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace\"/>" \
    "</tptz:Position>" \
    "</tptz:AbsoluteMove>" \
    SOAP_ENVELOPE_FOOTER

/* GetPresets - Returns PTZ presets */
#define SOAP_PTZ_GET_PRESETS \
    SOAP_ENVELOPE_HEADER \
    "<tptz:GetPresets>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "</tptz:GetPresets>" \
    SOAP_ENVELOPE_FOOTER

/* SetPreset - Creates or updates a PTZ preset */
#define SOAP_PTZ_SET_PRESET \
    SOAP_ENVELOPE_HEADER \
    "<tptz:SetPreset>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "<tptz:PresetName>HomePosition</tptz:PresetName>" \
    "<tptz:PresetToken>preset_1</tptz:PresetToken>" \
    "</tptz:SetPreset>" \
    SOAP_ENVELOPE_FOOTER

/* SetPreset without token (create new preset) */
#define SOAP_PTZ_SET_PRESET_NEW \
    SOAP_ENVELOPE_HEADER \
    "<tptz:SetPreset>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "<tptz:PresetName>NewPosition</tptz:PresetName>" \
    "</tptz:SetPreset>" \
    SOAP_ENVELOPE_FOOTER

/* GotoPreset - Moves to a saved preset */
#define SOAP_PTZ_GOTO_PRESET \
    SOAP_ENVELOPE_HEADER \
    "<tptz:GotoPreset>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "<tptz:PresetToken>preset_1</tptz:PresetToken>" \
    "<tptz:Speed>" \
    "<tt:PanTilt x=\"0.5\" y=\"0.5\" space=\"http://www.onvif.org/ver10/tptz/PanTiltSpaces/GenericSpeedSpace\"/>" \
    "<tt:Zoom x=\"0.5\" space=\"http://www.onvif.org/ver10/tptz/ZoomSpaces/ZoomGenericSpeedSpace\"/>" \
    "</tptz:Speed>" \
    "</tptz:GotoPreset>" \
    SOAP_ENVELOPE_FOOTER

/* RemovePreset - Deletes a PTZ preset */
#define SOAP_PTZ_REMOVE_PRESET \
    SOAP_ENVELOPE_HEADER \
    "<tptz:RemovePreset>" \
    "<tptz:ProfileToken>ptz_profile_1</tptz:ProfileToken>" \
    "<tptz:PresetToken>preset_to_delete</tptz:PresetToken>" \
    "</tptz:RemovePreset>" \
    SOAP_ENVELOPE_FOOTER

/* ============================================================================
 * Device Service Test Envelopes (4 operations)
 * ============================================================================ */

/* GetDeviceInformation - Returns device information (empty request) */
#define SOAP_DEVICE_GET_DEVICE_INFORMATION \
    SOAP_ENVELOPE_HEADER \
    "<tds:GetDeviceInformation/>" \
    SOAP_ENVELOPE_FOOTER

/* GetCapabilities - Returns device capabilities */
#define SOAP_DEVICE_GET_CAPABILITIES \
    SOAP_ENVELOPE_HEADER \
    "<tds:GetCapabilities>" \
    "<tds:Category>All</tds:Category>" \
    "</tds:GetCapabilities>" \
    SOAP_ENVELOPE_FOOTER

/* GetCapabilities with multiple categories */
#define SOAP_DEVICE_GET_CAPABILITIES_MULTI \
    SOAP_ENVELOPE_HEADER \
    "<tds:GetCapabilities>" \
    "<tds:Category>Media</tds:Category>" \
    "<tds:Category>PTZ</tds:Category>" \
    "<tds:Category>Imaging</tds:Category>" \
    "</tds:GetCapabilities>" \
    SOAP_ENVELOPE_FOOTER

/* GetSystemDateAndTime - Returns system date and time (empty request) */
#define SOAP_DEVICE_GET_SYSTEM_DATE_AND_TIME \
    SOAP_ENVELOPE_HEADER \
    "<tds:GetSystemDateAndTime/>" \
    SOAP_ENVELOPE_FOOTER

/* SystemReboot - Reboots the device (empty request) */
#define SOAP_DEVICE_SYSTEM_REBOOT \
    SOAP_ENVELOPE_HEADER \
    "<tds:SystemReboot/>" \
    SOAP_ENVELOPE_FOOTER

/* ============================================================================
 * Imaging Service Test Envelopes (2 operations)
 * ============================================================================ */

/* GetImagingSettings - Returns imaging settings */
#define SOAP_IMAGING_GET_IMAGING_SETTINGS \
    SOAP_ENVELOPE_HEADER \
    "<timg:GetImagingSettings>" \
    "<timg:VideoSourceToken>video_source_0</timg:VideoSourceToken>" \
    "</timg:GetImagingSettings>" \
    SOAP_ENVELOPE_FOOTER

/* SetImagingSettings - Sets imaging settings */
#define SOAP_IMAGING_SET_IMAGING_SETTINGS \
    SOAP_ENVELOPE_HEADER \
    "<timg:SetImagingSettings>" \
    "<timg:VideoSourceToken>video_source_0</timg:VideoSourceToken>" \
    "<timg:ImagingSettings>" \
    "<tt:Brightness>50.0</tt:Brightness>" \
    "<tt:Contrast>50.0</tt:Contrast>" \
    "<tt:Saturation>50.0</tt:Saturation>" \
    "<tt:Sharpness>50.0</tt:Sharpness>" \
    "<tt:BacklightCompensation>" \
    "<tt:Mode>OFF</tt:Mode>" \
    "<tt:Level>0.0</tt:Level>" \
    "</tt:BacklightCompensation>" \
    "<tt:WideDynamicRange>" \
    "<tt:Mode>OFF</tt:Mode>" \
    "<tt:Level>0.0</tt:Level>" \
    "</tt:WideDynamicRange>" \
    "</timg:ImagingSettings>" \
    "<timg:ForcePersistence>true</timg:ForcePersistence>" \
    "</timg:SetImagingSettings>" \
    SOAP_ENVELOPE_FOOTER

/* ============================================================================
 * Invalid/Malformed Request Test Envelopes
 * ============================================================================ */

/* Invalid XML - malformed XML syntax */
#define SOAP_INVALID_XML \
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" \
    "<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">" \
    "<s:Body>" \
    "<trt:GetProfiles>" \
    /* Missing closing tags */ \
    "</s:Body>"

/* Invalid namespace - wrong namespace URI */
#define SOAP_INVALID_NAMESPACE \
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" \
    "<s:Envelope " \
    "xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" " \
    "xmlns:trt=\"http://www.example.com/wrong/namespace\">" \
    "<s:Body>" \
    "<trt:GetProfiles/>" \
    "</s:Body>" \
    "</s:Envelope>"

/* Missing required parameter - GetStreamUri without ProfileToken */
#define SOAP_MISSING_REQUIRED_PARAM \
    SOAP_ENVELOPE_HEADER \
    "<trt:GetStreamUri>" \
    "<trt:StreamSetup>" \
    "<tt:Stream>RTP-Unicast</tt:Stream>" \
    "<tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport>" \
    "</trt:StreamSetup>" \
    "</trt:GetStreamUri>" \
    SOAP_ENVELOPE_FOOTER

/* Empty SOAP body */
#define SOAP_EMPTY_BODY \
    SOAP_ENVELOPE_HEADER \
    SOAP_ENVELOPE_FOOTER

/* Wrong operation name */
#define SOAP_WRONG_OPERATION \
    SOAP_ENVELOPE_HEADER \
    "<trt:NonExistentOperation/>" \
    SOAP_ENVELOPE_FOOTER

#endif /* SOAP_TEST_ENVELOPES_H */
