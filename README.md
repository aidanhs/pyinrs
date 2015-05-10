
```
git clone --recursive <repo>

cd cpython/Modules/zlib
CFLAGS="-fPIC" ./configure
make libz.a
cd -

cd cpython
sed -i 's/^#\(_struct\|operator\|_collections\|_heapq\|itertools\|binascii\) /\1 /' Modules/Setup
sed -i 's|^# zlib.*$|zlib zlibmodule.c -I./Modules/zlib -L./Modules/zlib -lz|' Modules/Setup
./configure --prefix=$(pwd)/dist --disable-shared
make OPT="-fPIC -O2"
make install
cd -

cd cpython/dist/lib/python2.7
LIBFILES=$(find . '(' -regex './\(distutils\|test\|idlelib\|lib2to3\|unittest\)' -o -regex '.*/tests*/.*' ')' -a -prune -o -name '*.pyo' -print)
rm -f $OLDPWD/libpython2.7.zip && for f in $LIBFILES; do zip $OLDPWD/libpython2.7.zip $f; done
cd -

export PKG_CONFIG_PATH=$(pwd)/cpython/dist/lib/pkgconfig
export PKG_CONFIG_ALL_STATIC=1
```

For a dynamic build:
```
cargo clean && cargo build
```

For a static build:
```
cargo clean && cargo build -p python27-sys
rm -f target/*/pyinrs && cargo rustc -- --emit obj -Z print-link-args | tail -n 1 | tr ' ' '\n' > linkargs
cat linkargs | grep -v '"\(-lpython2\.7\|-pie\|-Wl,.*-whole-archive\|-Wl,-B.*\)"' | sed 's/"-lgcc_s"/"-lgcc_eh"/' | sed 's/"cc"/"cc" "-static"/' | tr '\n' ' ' > cmd
sh cmd
```
