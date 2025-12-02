#!/usr/bin/env python

if __package__ in ("", None):
    import scgi_server as server
else:
    from . import scgi_server as server

if __name__ == "__main__":
    server.run()
