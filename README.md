Totally embedded python, including the stdlib.
Static build requires nothing but a writable /tmp (not even glibc).
Dynamic build is requires an appropriate glibc and supporting libraries - no need for Python installed.

Setup:

```
$ git clone --recursive <repo>
[...]
$ cd pyinrs
$ make prep
[...]
```

For a static build:
```
$ make static
[...]
$ ldd target/debug/pyinrs
        not a dynamic executable
$ du -h target/debug/pyinrs
12M     target/debug/pyinrs
$ docker run -it --rm -v $(pwd):/t -v /tmp:/tmp scratch /t/target/debug/pyinrs
Hello, python!
['/tmp/pyinrs-libpython2.7.zip', 'lib/python27.zip', 'lib/python2.7/', 'lib/python2.7/plat-linux2', 'lib/python2.7/lib-tk', 'lib/python2.7/lib-old', 'lib/python2.7/lib-dynload']
```

For a dynamic build:
```
$ make dynamic
[...]
$ ldd target/debug/pyinrs
        linux-vdso.so.1 =>  (0x00007fff0e39f000)
        libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f6ae87e1000)
        libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2 (0x00007f6ae85dd000)
        libutil.so.1 => /lib/x86_64-linux-gnu/libutil.so.1 (0x00007f6ae83d9000)
        libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f6ae8014000)
        /lib64/ld-linux-x86-64.so.2 (0x00007f6ae9095000)
        libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6 (0x00007f6ae7d0e000)
        libgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1 (0x00007f6ae7af7000)
$ du -h target/debug/pyinrs
9.6M    target/debug/pyinrs
$ docker run -it --rm -v $(pwd):/t ubuntu:14.04 /t/target/debug/pyinrs
Hello, python!
['/tmp/pyinrs-libpython2.7.zip', 'lib/python27.zip', 'lib/python2.7/', 'lib/python2.7/plat-linux2', 'lib/python2.7/lib-tk', 'lib/python2.7/lib-old', 'lib/python2.7/lib-dynload']
```

Note that the dynamic build does *not* dynamically link to Python or zlib.

You can do `make OPT=1 <target>` to enable a release build.

Make sure if you alter anything outside of the pyinrs `.rs` files you do a
`make clean`.
