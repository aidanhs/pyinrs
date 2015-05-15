export PKG_CONFIG_PATH=$(shell pwd)/cpython/dist/lib/pkgconfig
export PKG_CONFIG_ALL_STATIC=1

OPT ?= 0
CARGO_ARGS =
RUSTC_ARGS =
ifeq ($(OPT),1)
	CARGO_ARGS = --release
	RUSTC_ARGS = -O
endif

prep:
	cd cpython/Modules/zlib && \
		CFLAGS="-fPIC" ./configure && \
		make libz.a
	
	cd cpython && \
		./configure --prefix=$$(pwd)/dist --disable-shared && \
		sed -i 's/^#\(_struct\|operator\|_collections\|_heapq\|itertools\|binascii\) /\1 /' Modules/Setup && \
		sed -i 's|^#zlib.*$$|zlib zlibmodule.c -I./Modules/zlib -L./Modules/zlib -lz|' Modules/Setup && \
		make OPT="-fPIC -O2" && \
		make install
	
	rm -f libpython2.7.zip
	cd cpython/dist/lib/python2.7 && \
		LIBFILES=$$(find . '(' -regex './\(distutils\|test\|idlelib\|lib2to3\|unittest\)' -o -regex '.*/tests*/.*' ')' -a -prune -o -name '*.pyo' -print) && \
		for f in $$LIBFILES; do zip $$OLDPWD/libpython2.7.zip $$f; done
	cd -

clean:
	cargo clean

dynamic:
	cargo build $(CARGO_ARGS)

static:
	cargo build $(CARGO_ARGS) -p python27-sys
	CMD=$$(cargo rustc -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		tr ' ' '\n' | \
		grep -v '"\(-lpython2\.7\|-pie\|-Wl,.*-whole-archive\|-Wl,-B.*\)"' | \
		sed 's/"-lgcc_s"/"-lgcc_eh"/' | \
		sed 's/"cc"/"cc" "-static"/' | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"
