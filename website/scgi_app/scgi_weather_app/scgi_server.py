# -*- coding: utf-8 -*-

import socket
import os
import pwd
import grp


SOCK_NAME = "/run/lighttpd/scgi_app"
SOCK_UID = 33
SOCK_GID = 33

def get_uid_and_gid(uid_name='http', gid_name='http'):
    uid = pwd.getpwnam(uid_name).pw_uid
    gid = grp.getgrnam(gid_name).gr_gid
    return uid, gid

def create_socket():
    if os.path.exists(SOCK_NAME):
        os.remove(SOCK_NAME)

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCK_NAME)
    uid, gid = get_uid_and_gid()
    os.chown(SOCK_NAME, uid, gid)

    if os.getuid() == 0:
        # Remove group privileges
        os.setgroups([])

        # Try setting the new uid/gid
        os.setgid(gid)
        os.setuid(uid)

        # Ensure a very conservative umask
        old_umask = os.umask(0o077)
    return server

def run():
    server = create_socket()
    server.listen()

    while True:
        conn, addr = server.accept()
    
        leftover = b""
        while True:
            data = conn.recv(1024)
            if not data:
                break

            data = leftover + data
            parts = data.split(b':')
            if len(parts) < 2:
                leftover = data
                continue
            length = int(parts[0].decode("ascii"))
            if len(parts[1]) <= length:
                leftover = data
                continue

            hdr = parts[1][:length]
            rest = parts[1][length+1:]
           
            hdr_dict = {}
            tokens = hdr.split(b'\0')
            idx = 0;
            end = 2 * (len(tokens) // 2)
            while idx < end:
                name = tokens[idx]
                value = tokens[idx+1]
                hdr_dict[name] = value
                idx += 2
            print(hdr_dict)
            request = hdr_dict[b'REQUEST_URI'].decode("ascii")


