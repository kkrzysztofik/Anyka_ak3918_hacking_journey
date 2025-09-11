#!/bin/sh
# Login validation script
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

echo "Content-type: text/html"
echo ""
creds=$(echo "$QUERY_STRING" | awk '{split($0,a,"&")} END{print a[1]}' | awk '{split($0,a,"=")} END{print a[2]}')
hash_input="$(readline 2 /data/webui.hash)$creds"
if [ "$(echo $hash_input | md5sum )" = "$(readline 1 /data/webui.hash)" ]; then
token=$(rand_token)
cat <<EOT
<html>
    <head>
        <meta http-equiv="refresh" content="0; url=/cgi-bin/legacy/webui?token=$token" />
    </head>
    <body>
        <a href="/cgi-bin/legacy/webui">Legacy WebUI</a>
    </body>
</html>
EOT
mkdir -p /mnt/tmp 2>/dev/null || true
echo "$token">/mnt/tmp/token.txt

else
    if [ -f /data/www/index.html ]; then
        . /data/www/cgi-bin/footer
    else
        . /mnt/anyka_hack/web_interface/www/cgi-bin/footer
    fi
fi
