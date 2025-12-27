#!/bin/bash
# generate_gsoap.sh

# Set up gSOAP environment
export SOAPCPP2=/usr/bin/soapcpp2
export WSDL2H=/usr/bin/wsdl2h

# Get the script directory to find relative paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WSDL_DIR="${SCRIPT_DIR}/../wsdl"
GENERATED_DIR="${SCRIPT_DIR}/../src/generated"

# Create output directory
mkdir -p "${GENERATED_DIR}"

# Validate required tools are available
if ! command -v wsdl2h &> /dev/null; then
    echo "Error: wsdl2h not found. Please install gSOAP tools." >&2
    exit 1
fi

if ! command -v soapcpp2 &> /dev/null; then
    echo "Error: soapcpp2 not found. Please install gSOAP tools." >&2
    exit 1
fi

# Validate required schema files exist (check from WSDL directory)
REQUIRED_SCHEMAS=(
    "schemas/ws-addr.xsd"
    "schemas/soap-envelope.xsd"
    "schemas/xml.xsd"
    "schemas/wsn-b-2.xsd"
    "schemas/wsn-t-1.xsd"
    "schemas/wsrf-bf-2.xsd"
    "schemas/xmlmime.xsd"
    "schemas/xop-include.xsd"
)

# Change to WSDL directory first to check schema files
cd "${WSDL_DIR}"

for schema in "${REQUIRED_SCHEMAS[@]}"; do
    if [[ ! -f "${schema}" ]]; then
        echo "Error: Required schema file ${schema} not found in ${WSDL_DIR}." >&2
        exit 1
    fi
done

echo "All required schema files found. Proceeding with code generation..."

# Use the typemap.dat file from the wsdl directory
TYPEMAP_FILE="${WSDL_DIR}/typemap.dat"


# Generate header file from WSDL files in wsdl/ directory
# Run from WSDL directory so relative schema paths work
# Use local schema cache to avoid internet dependencies
# The -N flags map namespace URIs to local schema files
# Added -p prefix to avoid naming conflicts and -F for inheritance
echo "Generating header file from WSDL files..."
if wsdl2h -o "${GENERATED_DIR}/onvif.h" \
    -c -s -t "${TYPEMAP_FILE}" -F -P \
    -x -n onvif \
    -I schemas \
    -N "http://www.w3.org/2005/08/addressing" "schemas/ws-addr.xsd" \
    -N "http://www.w3.org/2003/05/soap-envelope" "schemas/soap-envelope.xsd" \
    -N "http://www.w3.org/2001/XMLSchema" "schemas/xml.xsd" \
    -N "http://docs.oasis-open.org/wsn/b-2" "schemas/wsn-b-2.xsd" \
    -N "http://docs.oasis-open.org/wsn/t-1" "schemas/wsn-t-1.xsd" \
    -N "http://docs.oasis-open.org/wsrf/bf-2" "schemas/wsrf-bf-2.xsd" \
    -N "http://www.w3.org/2005/05/xmlmime" "schemas/xmlmime.xsd" \
    -N "http://www.w3.org/2004/08/xop/include" "schemas/xop-include.xsd" \
    devicemgmt.wsdl \
    media.wsdl \
    ptz.wsdl \
    imaging.wsdl; then
    echo "✅ Header file generation completed successfully!"
else
    echo "❌ Header file generation failed!"
    exit 1
fi

# Generate C code in the generated/ directory
echo "Generating C code from header file..."
if soapcpp2 -2 -c -L -S -x \
    -I/usr/share/gsoap/import \
    -d "${GENERATED_DIR}" \
    "${GENERATED_DIR}/onvif.h"; then
    echo "✅ Code generation completed successfully!"
    echo "Generated files are in: ${GENERATED_DIR}"
else
    echo "❌ Code generation failed!"
    exit 1
fi
