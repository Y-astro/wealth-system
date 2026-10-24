from pathlib import Path
import http.client
import shutil
import socket
import ssl
import subprocess
import time

home = Path.home()
path = home/'.config/caddy/Caddyfile'
caddy = '/data/data/com.termux/files/usr/bin/caddy'
source = path.read_text()
begin = source.index('http://:8000 {')
end = source.index('https://:8443 {',begin)
hosts = ['192.168.1.8','192.168.1.10','100.75.159.53','redmi-k20-pro.tail939a82.ts.net']
public = home/'.config/caddy/public'
public.mkdir(parents=True,exist_ok=True)
shutil.copy2('/storage/emulated/0/Temp/RedmiHomeServerCA.crt',public/'RedmiHomeServerCA.crt')
cleartext = ', '.join('http://'+host+':8000' for host in hosts)+''' {
    handle /wealth-ca.crt {
        root * /data/data/com.termux/files/home/.config/caddy/public
        rewrite * /RedmiHomeServerCA.crt
        file_server
    }
    handle {
        route {
            @unsafe not method GET HEAD
            respond @unsafe "HTTPS required. Reopen the HTTPS portal before submitting credentials or files." 400
            redir https://{host}:8443{uri} 308
        }
    }
}

'''
source = source[:begin]+cleartext+source[end:]
source = source.replace('https://:8443 {\n\ttls internal {\n\t\ton_demand\n\t}',
    ', '.join('https://'+host+':8443' for host in hosts)+' {\n\ttls internal')
assert 'on_demand' not in source
prefix,wealth = source.split('# Wealthfolio is a separate origin',1)
wealth = wealth.replace('Cache-Control "no-store, private, max-age=0"','?Cache-Control "no-store, private, max-age=0"',1)
wealth = wealth.replace('        Pragma "no-cache"\n','',1)
wealth = wealth.replace('handle /wealth/api/* {\n        route {','''handle /wealth/api/* {
        route {
            header >Cache-Control "no-store, private, max-age=0"
            header >Pragma "no-cache"''',1)
source = prefix+'# Wealthfolio is a separate origin'+wealth
# Uploaded active documents must not execute with the portal's origin or its
# administrator terminal permissions. Ordinary images, video and PDFs stream
# through the existing gateway unchanged.
raw = '''
    handle /drive/api/raw/* {
        forward_auth 127.0.0.1:8001 {
            uri /api/auth-check
            copy_headers X-Forwarded-User
        }
        reverse_proxy 127.0.0.1:8081 {
            flush_interval -1
            @activeContent {
                header Content-Type text/html*
                header Content-Type image/svg+xml*
                header Content-Type application/xhtml+xml*
                header Content-Type application/xml*
                header Content-Type text/xml*
                header Content-Type application/javascript*
                header Content-Type text/javascript*
            }
            handle_response @activeContent {
                header {
                    >Content-Security-Policy "sandbox; default-src 'none'; style-src 'unsafe-inline'; img-src data:"
                    >Content-Disposition attachment
                    >X-Content-Type-Options nosniff
                }
                copy_response
            }
            transport http {
                keepalive 90s
                response_header_timeout 300s
                read_buffer 32768
                write_buffer 32768
            }
        }
    }
'''
assert source.count('\thandle /drive* {') == 1
source = source.replace('\thandle /drive* {',raw+'\n\thandle /drive* {',1)
candidate = path.with_name('Caddyfile.https-final')
candidate.write_text(source)
subprocess.run([caddy,'validate','--config',str(candidate),'--adapter','caddyfile'],check=True)
backup = path.with_name('Caddyfile.before-https-'+str(int(time.time())))
shutil.copy2(path,backup)
context = ssl.create_default_context(cafile=str(public/'RedmiHomeServerCA.crt'))

def check_tls(port, uri, expected):
    # IP literals are not sent as SNI. Use the configured DNS name while
    # connecting locally so the certificate matches and is fully verified.
    hostname = 'redmi-k20-pro.tail939a82.ts.net'
    for attempt in range(5):
        raw = socket.create_connection(('127.0.0.1',port),timeout=8)
        try:
            stream = context.wrap_socket(raw,server_hostname=hostname)
            break
        except ssl.SSLCertVerificationError: raise
        except ssl.SSLError:
            raw.close()
            if attempt == 4: raise
            time.sleep(1)
    stream.sendall(('GET '+uri+' HTTP/1.1\r\nHost: '+hostname+':'+str(port)+'\r\nConnection: close\r\n\r\n').encode())
    response = http.client.HTTPResponse(stream)
    response.begin()
    assert response.status == expected,(port,uri,response.status)
    response.close()
    stream.close()

try:
    shutil.copy2(candidate,path)
    subprocess.run([caddy,'reload','--config',str(path),'--adapter','caddyfile'],check=True)
    check_tls(8443,'/login',200)
    check_tls(8444,'/wealth/api/v1/statements/library',401)
    connection = http.client.HTTPConnection('127.0.0.1',8000,timeout=8)
    connection.request('GET','/login',headers={'Host':'100.75.159.53:8000'})
    response=connection.getresponse()
    assert response.status == 308 and response.getheader('Location') == 'https://100.75.159.53:8443/login'
    response.read()
    connection.close()
    connection = http.client.HTTPConnection('127.0.0.1',8000,timeout=8)
    connection.request('POST','/api/login',b'{}',headers={'Host':'100.75.159.53:8000','Content-Type':'application/json'})
    assert connection.getresponse().status == 400
    connection.close()
except Exception:
    shutil.copy2(backup,path)
    subprocess.run([caddy,'reload','--config',str(path),'--adapter','caddyfile'],check=True)
    raise
print('Verified HTTPS portal and protected Wealthfolio API; HTTP forms disabled; unknown TLS names disabled.')
