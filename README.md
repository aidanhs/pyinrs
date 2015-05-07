
```
git clone --recursive <repo>
cd cpython
./configure --prefix=$(pwd)/dist --enable-shared # adds -fPIC
make
make install
cd ..
PKG_CONFIG_PATH=$(pwd)/cpython/dist/lib/pkgconfig cargo build
```
