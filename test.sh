#!/usr/bin/env bash
# Test the full upload and stage 1 processing pipeline

set -e

echo "🧪 Testing Full Upload → Media Conversion Pipeline"
echo "=========================================="
echo ""

# Test data
USER_ID="test_user_$(date +%s)"
AGE=25
ETHNICITY="caucasian"
SEX="M"
HEIGHT="180cm"
WEIGHT=75
EMAIL="test@example.com"
FRONT_VIDEO="front.MOV"
SIDE_VIDEO="side.MOV"

echo "📋 Test Parameters:"
echo "  User ID: $USER_ID"
echo "  Age: $AGE"
echo "  Email: $EMAIL"
echo ""
echo "📤 Uploading videos to backend..."
echo ""

# Upload using multipart form data
RESPONSE=$(curl -X POST http://localhost:3000/api/v1/upload \
  -F "uid=$USER_ID" \
  -F "age=$AGE" \
  -F "ethnicity=$ETHNICITY" \
  -F "sex=$SEX" \
  -F "height=$HEIGHT" \
  -F "weight=$WEIGHT" \
  -F "email=$EMAIL" \
  -F "fileuploadfront=@$FRONT_VIDEO" \
  -F "fileuploadside=@$SIDE_VIDEO" \
  -w "\n%{http_code}" \
  -s)

# Extract status code (last line)
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | head -n -1)

echo "HTTP Status: $HTTP_CODE"

if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ Upload successful!"
    echo ""
    echo "📊 Monitoring Media Conversion processing..."
    echo "   Run: docker compose logs -f media-conversion"
    echo ""
    echo "🔍 Check backend logs:"
    echo "   Run: docker compose logs -f backend"
    echo ""
    echo "Job ID: ${USER_ID}_0"
else
    echo "❌ Upload failed!"
    echo "Response: $BODY"
    exit 1
fi
