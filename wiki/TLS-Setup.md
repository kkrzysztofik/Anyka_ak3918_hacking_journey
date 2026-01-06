# TLS Setup Guide

## Overview

This guide explains how to configure TLS (Transport Layer Security) for the ONVIF server. TLS is **strongly recommended** when authentication is enabled to protect credentials in transit.

## Quick Start

### 1. Generate Self-Signed Certificates (Development/Testing)

For development and testing purposes, you can generate self-signed certificates:

```bash
# Generate private key
openssl genrsa -out server-key.pem 2048

# Generate self-signed certificate (valid for 365 days)
openssl req -new -x509 -key server-key.pem -out server-cert.pem -days 365

# You'll be prompted for certificate information:
# - Country Name (2 letter code): US
# - State or Province Name: California
# - Locality Name: San Francisco
# - Organization Name: My Camera System
# - Organizational Unit Name: Security
# - Common Name: 192.168.1.100  (use your camera's IP address)
# - Email Address: admin@example.com
```

### 2. Configure TLS in config.toml

Edit your `config.toml` file:

```toml
[server]
bind_address = "0.0.0.0"
port = 8443  # Standard HTTPS port (or use 443 for production)
tls_enabled = true
tls_cert_path = "/path/to/server-cert.pem"
tls_key_path = "/path/to/server-key.pem"
auth_enabled = true  # Enable authentication with TLS

[server.security]
require_tls_for_auth = true  # Reject auth over non-TLS connections
```

### 3. Start the Server

```bash
./onvif-server --config config.toml
```

The server will now accept HTTPS connections on the configured port.

## Production Deployment

### Option 1: Pre-Generated Certificates (Recommended for Embedded)

Since the Anyka camera runs embedded Linux without certbot, generate certificates on your development machine and transfer them to the camera via FTP:

**On your development machine:**

```bash
# For Let's Encrypt (if you have a domain):
sudo apt-get install certbot
sudo certbot certonly --standalone -d camera.example.com

# Copy certificates to a temporary location
sudo cp /etc/letsencrypt/live/camera.example.com/fullchain.pem ~/camera-certs/
sudo cp /etc/letsencrypt/live/camera.example.com/privkey.pem ~/camera-certs/
sudo chown $USER:$USER ~/camera-certs/*.pem

# Transfer to camera via FTP
ftp 192.168.1.100
# At FTP prompt:
# > cd /etc/onvif
# > put fullchain.pem
# > put privkey.pem
# > quit
```

**On the camera** (via telnet), update `config.toml`:

```toml
[server]
tls_cert_path = "/etc/onvif/fullchain.pem"
tls_key_path = "/etc/onvif/privkey.pem"
```

**Certificate Renewal:**

- Renew on your development machine: `sudo certbot renew`
- Transfer updated certificates to the camera via FTP
- Restart the ONVIF server via telnet: `killall onvif-server && /usr/bin/onvif-server &`

### Option 2: Reverse Proxy (Recommended for Production)

Use a reverse proxy (nginx, Caddy) on a more capable server to handle TLS:

**On a separate server (not the camera):**

```nginx
# nginx configuration
server {
    listen 443 ssl;
    server_name camera.example.com;

    ssl_certificate /etc/letsencrypt/live/camera.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/camera.example.com/privkey.pem;

    location / {
        proxy_pass http://192.168.1.100:8080;  # Camera's HTTP endpoint
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

**Benefits:**

- Automatic certificate renewal on the proxy server
- No TLS overhead on the embedded camera
- Easier to manage and update

### Option 3: Self-Signed Certificates (Testing/Internal Use)

For internal networks or testing, use self-signed certificates:

```bash
# Generate on development machine
openssl genrsa -out server-key.pem 2048
openssl req -new -x509 -key server-key.pem -out server-cert.pem -days 365

# Transfer to camera via FTP
ftp 192.168.1.100
# At FTP prompt:
# > cd /etc/onvif
# > put server-cert.pem
# > put server-key.pem
# > quit
```

### Using Commercial Certificates

If you have a commercial certificate from a CA (Certificate Authority):

1. Obtain your certificate and private key from your CA
2. Ensure the certificate is in PEM format
3. Configure paths in `config.toml`

## Security Warnings

> **⚠️ Never use authentication without TLS in production!**
>
> ONVIF 24.12 uses WS-Security with SHA-1/MD5 digest authentication. While this provides replay protection via nonces and timestamps, credentials can be intercepted and cracked if transmitted over unencrypted HTTP.

> **⚠️ Self-Signed Certificates**
>
> Self-signed certificates will trigger security warnings in browsers and ONVIF clients. They are acceptable for:
>
> - Development and testing
> - Internal networks with manual certificate trust
>
> For production, use certificates from a trusted CA (Let's Encrypt, DigiCert, etc.)

> **⚠️ File Permissions**
>
> Protect your private key file:
>
> ```bash
> chmod 600 /path/to/server-key.pem
> chown camera-user:camera-group /path/to/server-key.pem
> ```

## Testing TLS Configuration

### Using curl

```bash
# Test HTTPS connection
curl -k https://192.168.1.100:8443/onvif/device_service \
  -H "Content-Type: text/xml" \
  -d '<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
        <s:Body><GetDeviceInformation/></s:Body>
      </s:Envelope>'

# -k flag skips certificate verification (for self-signed certs)
```

### Using ONVIF Device Manager

1. Add device with HTTPS URL: `https://192.168.1.100:8443`
2. If using self-signed cert, you may need to accept the certificate warning
3. Enter credentials if authentication is enabled

## Troubleshooting

### "Certificate verify failed" Error

**Cause**: Client doesn't trust the certificate (common with self-signed certs)

**Solutions**:

- Use a certificate from a trusted CA
- Add self-signed cert to client's trust store
- Use `-k` flag with curl (testing only)

### "Connection refused" Error

**Cause**: Server not listening on HTTPS port

**Check**:

```bash
# Verify server is listening
netstat -tlnp | grep 8443

# Check server logs
journalctl -u onvif-server -f
```

### "Permission denied" Error

**Cause**: Server can't read certificate/key files

**Fix**:

```bash
# Check file permissions
ls -l /path/to/server-*.pem

# Fix permissions
chmod 644 /path/to/server-cert.pem
chmod 600 /path/to/server-key.pem
chown camera-user:camera-group /path/to/server-*.pem
```

## Advanced Configuration

### Custom Cipher Suites

For enhanced security, you can configure specific TLS cipher suites (requires code modification):

```rust
// In src/onvif/server.rs
let tls_config = rustls::ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(certs, key)?;
```

### Client Certificate Authentication (mTLS)

For mutual TLS authentication (requires code modification):

```rust
let tls_config = rustls::ServerConfig::builder()
    .with_safe_defaults()
    .with_client_cert_verifier(verifier)
    .with_single_cert(certs, key)?;
```

## References

- [ONVIF Security Best Practices](https://www.onvif.org/specs/core/ONVIF-Security-Guide.pdf)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [OpenSSL Certificate Generation](https://www.openssl.org/docs/man1.1.1/man1/req.html)
- [Rustls TLS Library](https://docs.rs/rustls/)

## Support

For issues or questions:

- Check server logs: `journalctl -u onvif-server -f`
- Review configuration: `cat /etc/onvif/config.toml`
- File an issue: [GitHub Issues](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/issues)
