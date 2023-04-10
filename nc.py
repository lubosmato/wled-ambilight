from socket import socket,AF_INET,SOCK_DGRAM,SO_REUSEADDR,SOL_SOCKET
from time import sleep,ctime
import sys

if len(sys.argv)>2:
    localIP = sys.argv[1]
    localPort = int(sys.argv[2])
else:
    print("Space-separated IP and Port are required")
    exit()

print(f"listening on {localIP}:{localPort}")
bufSize = 1500

sock = socket(family=AF_INET, type=SOCK_DGRAM)
sock.setsockopt(SOL_SOCKET,SO_REUSEADDR, 1)
sock.bind((localIP, localPort))

while True:
    message, ipport = sock.recvfrom(bufSize)
    print(ctime(), f"[{ipport[0]}:{ipport[1]}]",str(message)[2:-1])
