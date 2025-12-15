# API Contract: Device Service

**Service Endpoint**: `POST /onvif/device_service`

All operations use SOAP 1.2 with WS-UsernameToken authentication.

---

## GetDeviceInformation

Retrieves device identity information.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation`

### GetDeviceInformation Request

```xml
<tds:GetDeviceInformation />
```

### GetDeviceInformation Response

```xml
<tds:GetDeviceInformationResponse>
  <tds:Manufacturer>string</tds:Manufacturer>
  <tds:Model>string</tds:Model>
  <tds:FirmwareVersion>string</tds:FirmwareVersion>
  <tds:SerialNumber>string</tds:SerialNumber>
  <tds:HardwareId>string</tds:HardwareId>
</tds:GetDeviceInformationResponse>
```

### Frontend Mapping

```typescript
interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
}
```

---

## GetSystemDateAndTime

Retrieves current date/time configuration.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/GetSystemDateAndTime`

### GetSystemDateAndTime Request

```xml
<tds:GetSystemDateAndTime />
```

### GetSystemDateAndTime Response

```xml
<tds:GetSystemDateAndTimeResponse>
  <tds:SystemDateAndTime>
    <tt:DateTimeType>NTP|Manual</tt:DateTimeType>
    <tt:DaylightSavings>boolean</tt:DaylightSavings>
    <tt:TimeZone>
      <tt:TZ>string</tt:TZ>
    </tt:TimeZone>
    <tt:UTCDateTime>
      <tt:Time>
        <tt:Hour>int</tt:Hour>
        <tt:Minute>int</tt:Minute>
        <tt:Second>int</tt:Second>
      </tt:Time>
      <tt:Date>
        <tt:Year>int</tt:Year>
        <tt:Month>int</tt:Month>
        <tt:Day>int</tt:Day>
      </tt:Date>
    </tt:UTCDateTime>
  </tds:SystemDateAndTime>
</tds:GetSystemDateAndTimeResponse>
```

---

## SetSystemDateAndTime

Sets date/time configuration.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/SetSystemDateAndTime`

### SetSystemDateAndTime Request

```xml
<tds:SetSystemDateAndTime>
  <tds:DateTimeType>NTP|Manual</tds:DateTimeType>
  <tds:DaylightSavings>boolean</tds:DaylightSavings>
  <tds:TimeZone>
    <tt:TZ>string</tt:TZ>
  </tds:TimeZone>
  <tds:UTCDateTime><!-- optional, for Manual mode -->
    <tt:Time>...</tt:Time>
    <tt:Date>...</tt:Date>
  </tds:UTCDateTime>
</tds:SetSystemDateAndTime>
```

### SetSystemDateAndTime Response

```xml
<tds:SetSystemDateAndTimeResponse />
```

---

## GetNetworkInterfaces

Retrieves network interface configuration.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/GetNetworkInterfaces`

### GetNetworkInterfaces Request

```xml
<tds:GetNetworkInterfaces />
```

### GetNetworkInterfaces Response

```xml
<tds:GetNetworkInterfacesResponse>
  <tds:NetworkInterfaces token="string">
    <tt:Enabled>boolean</tt:Enabled>
    <tt:Info>
      <tt:Name>string</tt:Name>
      <tt:HwAddress>string</tt:HwAddress>
    </tt:Info>
    <tt:IPv4>
      <tt:Enabled>boolean</tt:Enabled>
      <tt:Config>
        <tt:DHCP>boolean</tt:DHCP>
        <tt:Manual>
          <tt:Address>string</tt:Address>
          <tt:PrefixLength>int</tt:PrefixLength>
        </tt:Manual>
      </tt:Config>
    </tt:IPv4>
  </tds:NetworkInterfaces>
</tds:GetNetworkInterfacesResponse>
```

---

## SystemReboot

Initiates device reboot.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/SystemReboot`

### SystemReboot Request

```xml
<tds:SystemReboot />
```

### SystemReboot Response

```xml
<tds:SystemRebootResponse>
  <tds:Message>string</tds:Message>
</tds:SystemRebootResponse>
```

---

## SetSystemFactoryDefault

Resets device to factory defaults.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/SetSystemFactoryDefault`

### SetSystemFactoryDefault Request

```xml
<tds:SetSystemFactoryDefault>
  <tds:FactoryDefault>Hard|Soft</tds:FactoryDefault>
</tds:SetSystemFactoryDefault>
```

### SetSystemFactoryDefault Response

```xml
<tds:SetSystemFactoryDefaultResponse />
```

---

## Error Handling

All operations may return SOAP Faults:

```xml
<soap:Fault>
  <soap:Code>
    <soap:Value>soap:Sender|soap:Receiver</soap:Value>
    <soap:Subcode>
      <soap:Value>ter:InvalidArgs|ter:NotAuthorized|ter:ActionNotSupported</soap:Value>
    </soap:Subcode>
  </soap:Code>
  <soap:Reason>
    <soap:Text xml:lang="en">Human-readable error message</soap:Text>
  </soap:Reason>
</soap:Fault>
```

### Common Fault Codes

| Subcode | Meaning | Frontend Action |
|---------|---------|-----------------|
| `ter:NotAuthorized` | Insufficient permissions | Show "Permission denied" toast |
| `ter:InvalidArgs` | Invalid request parameters | Show validation error |
| `ter:ActionNotSupported` | Operation not supported | Hide/disable feature |
