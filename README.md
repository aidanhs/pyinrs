
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
cd ..

cd cpython/dist/lib/python2.7
LIBFILES=$(find . '(' -regex './\(distutils\|test\|idlelib\|lib2to3\|unittest\)' -o -regex '.*/tests*/.*' ')' -a -prune -o -name '*.pyo' -print)
for f in $LIBFILES; do zip $OLDPWD/libpython2.7.zip $f; done
cd -

PKG_CONFIG_PATH=$(pwd)/cpython/dist/lib/pkgconfig PKG_CONFIG_ALL_STATIC=1 cargo build
```
