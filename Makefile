export PKG_CONFIG_PATH=$(shell pwd)/cpython/dist/lib/pkgconfig
export PKG_CONFIG_ALL_STATIC=1

OPT ?= 0
WRAPLIBC ?= 1

CARGO_ARGS =
RUSTC_ARGS =
ifeq ($(OPT),1)
	CARGO_ARGS += --release
	RUSTC_ARGS += -O
endif

WRAP_CMD = cat
ifeq ($(WRAPLIBC),1)
	WRAP_SYMS = read pread pread64 pwrite pwrite64 open open64 lseek lseek64 \
		stat stat64 __xstat __xstat64 \
		lstat lstat64 __lxstat __lxstat64 \
		fstat fstat64 __fxstat __fxstat64 \
		fclose fopen fopen64 fdopen fdopen64 freopen freopen64 \
		fread fread64 fwrite fwrite64 \
		fgetc fgets getc _IO_getc ungetc \
		fseek fseek64 fseeko fseeko64 ftell ftell64 ftello ftello64 rewind \
		fgetpos fgetpos64 fsetpos fsetpos64 clearerr feof ferror fileno
	WRAP_CMD = sed 's/"cc"/"cc" $(foreach sym,$(WRAP_SYMS),-Wl,--wrap,$(sym))/'
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
	cargo build $(CARGO_ARGS) -p python27-sys
	CMD=$$(cargo rustc -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		tr ' ' '\n' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"

static:
	cargo build $(CARGO_ARGS) -p python27-sys
	CMD=$$(cargo rustc -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		tr ' ' '\n' | \
		grep -v '"\(-lpython2\.7\|-pie\|-Wl,.*-whole-archive\|-Wl,-B.*\)"' | \
		sed 's/"-lgcc_s"/"-lgcc_eh"/' | \
		sed 's/"cc"/"cc" "-static"/' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"
