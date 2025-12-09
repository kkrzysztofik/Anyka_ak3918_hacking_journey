#!/bin/bash

# Final script to extract XML content from ONVIF pcapng file
# This script extracts all HTTP requests and responses containing XML/SOAP content

PCAP_FILE="/home/kmk/anyka-dev/onvif.pcapng"
OUTPUT_DIR="/home/kmk/anyka-dev/cap"

echo "Final extraction of XML content from $PCAP_FILE..."

# Create subdirectories for requests and responses
mkdir -p "$OUTPUT_DIR/requests"
mkdir -p "$OUTPUT_DIR/responses"

# Function to extract action from XML content
extract_action() {
    local file="$1"
    local action=""

    # Try to extract action from XML content
    action=$(grep -o '<[^>]*xmlns="http://www.onvif.org/ver10/[^"]*"' "$file" | head -1 | sed 's/.*xmlns="http:\/\/www.onvif.org\/ver10\/\([^"]*\)".*/\1/')

    if [ -z "$action" ]; then
        # Try to extract from element names
        action=$(grep -o '<[A-Za-z][A-Za-z0-9]*[^>]*>' "$file" | grep -v 's:Envelope\|s:Body\|s:Header' | head -1 | sed 's/<\([A-Za-z][A-Za-z0-9]*\).*/\1/')
    fi

    if [ -z "$action" ]; then
        action="unknown"
    else
        # Clean up action name
        action=$(echo "$action" | sed 's|/|_|g' | sed 's|\.wsdl||g' | sed 's|wsdl||g')
    fi

    echo "$action"
}

# Extract HTTP requests
echo "Extracting HTTP requests..."
request_count=0
tshark -r "$PCAP_FILE" -Y "http.request.method and http.content_type matches \"xml\"" -T fields -e frame.number -e http.request.method -e http.content_type | while IFS=$'\t' read -r frame_num method content_type; do
    if [ -n "$frame_num" ]; then
        request_count=$((request_count + 1))

        # Extract XML content
        xml_content=$(tshark -r "$PCAP_FILE" -Y "frame.number == $frame_num" -T fields -e http.file_data)

        if [ -n "$xml_content" ] && [ "$xml_content" != "" ]; then
            # Create temporary file to analyze content
            temp_file="/tmp/temp_${frame_num}.xml"
            if echo "$xml_content" | grep -q "^[0-9a-fA-F]*$" && [ ${#xml_content} -gt 10 ]; then
                echo "$xml_content" | xxd -r -p > "$temp_file"
            else
                echo "$xml_content" > "$temp_file"
            fi

            # Extract action
            action=$(extract_action "$temp_file")

            # Create filename
            filename="request_$(printf "%03d" $request_count)_${action}_${frame_num}.xml"
            output_file="$OUTPUT_DIR/requests/$filename"

            # Copy to final location
            cp "$temp_file" "$output_file"
            rm -f "$temp_file"

            echo "Extracted request: $filename"
        fi
    fi
done

# Extract HTTP responses
echo "Extracting HTTP responses..."
response_count=0
tshark -r "$PCAP_FILE" -Y "http.response.code and http.content_type matches \"xml\"" -T fields -e frame.number -e http.response.code -e http.content_type | while IFS=$'\t' read -r frame_num response_code content_type; do
    if [ -n "$frame_num" ]; then
        response_count=$((response_count + 1))

        # Extract XML content
        xml_content=$(tshark -r "$PCAP_FILE" -Y "frame.number == $frame_num" -T fields -e http.file_data)

        if [ -n "$xml_content" ] && [ "$xml_content" != "" ]; then
            # Create temporary file to analyze content
            temp_file="/tmp/temp_${frame_num}.xml"
            if echo "$xml_content" | grep -q "^[0-9a-fA-F]*$" && [ ${#xml_content} -gt 10 ]; then
                echo "$xml_content" | xxd -r -p > "$temp_file"
            else
                echo "$xml_content" > "$temp_file"
            fi

            # Extract action
            action=$(extract_action "$temp_file")

            # Create filename
            filename="response_$(printf "%03d" $response_count)_${action}_${response_code}_${frame_num}.xml"
            output_file="$OUTPUT_DIR/responses/$filename"

            # Copy to final location
            cp "$temp_file" "$output_file"
            rm -f "$temp_file"

            echo "Extracted response: $filename"
        fi
    fi
done

echo "Final XML extraction completed!"
echo "Files saved in:"
echo "  Requests: $OUTPUT_DIR/requests/"
echo "  Responses: $OUTPUT_DIR/responses/"
