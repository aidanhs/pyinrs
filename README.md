
```
git clone --recursive <repo>
cd cpython
./configure --prefix=$(pwd)/dist --disable-shared #--enable-shared # adds -fPIC
#make
make OPT="-fpic -O2"
make install
cd ..
PKG_CONFIG_PATH=$(pwd)/cpython/dist/lib/pkgconfig PKG_CONFIG_ALL_STATIC=1 cargo build
```
