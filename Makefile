export PKG_CONFIG_PATH=$(shell pwd)/cpython/dist/lib/pkgconfig
export PKG_CONFIG_ALL_STATIC=1

default:
	@echo "Choose one of 'prep', 'static', 'dynamic', 'clean'"

OPT ?= 0
MODE ?= wrap

CARGO_ARGS =
FEAT = --features "$(MODE)"
RUSTC_ARGS = --cfg 'feature="$(MODE)"'
ifeq ($(OPT),1)
	CARGO_ARGS += --release
	RUSTC_ARGS += -O
endif

WRAP_CMD = cat
ifeq ($(MODE),wrap)
	WRAP_SYMS = \
		read pread pread64 pwrite pwrite64 open open64 openat openat64 \
		creat creat64 lseek lseek64 \
		stat stat64 __xstat __xstat64 \
		lstat lstat64 __lxstat __lxstat64 \
		fstat fstat64 __fxstat __fxstat64 \
		opendir fdopendir closedir readdir readdir64 readdir_r readdir_r64 \
		rewinddir seekdir telldir \
		fclose fopen fopen64 fdopen fdopen64 freopen freopen64 \
		fread fread64 fwrite fwrite64 \
		fgetc fgets getc _IO_getc ungetc \
		fseek fseek64 fseeko fseeko64 ftell ftell64 ftello ftello64 rewind \
		fgetpos fgetpos64 fsetpos fsetpos64 clearerr feof ferror fileno \
		flockfile ftrylockfile funlockfile
	WRAP_CMD = sed 's/"cc"/"cc" $(foreach sym,$(WRAP_SYMS),-Wl,--wrap,$(sym))/'
endif

checkmode:
	[ "$(MODE)" = dump -o "$(MODE)" = wrap ]

prep:
	cd cpython/Modules/zlib && \
		CFLAGS="-fPIC" ./configure && \
		make libz.a
	
	cd cpython && \
		./configure --prefix=$$(pwd)/dist --disable-shared && \
		sed -i 's/^#define \(HAVE_GETC_UNLOCKED\).*/#undef \1/' pyconfig.h && \
		sed -i 's/^#\(array\|cmath\|math\|_struct\|time\|operator\|_random\|_collections\|_heapq\|itertools\|_functools\|datetime\|unicodedata\|_io\|fcntl\|select\|_socket\|termios\|resource\|_md5\|_sha\|_sha256\|_sha512\|binascii\|cStringIO\|cPickle\) /\1 /' Modules/Setup && \
		sed -i 's|^#zlib.*$$|zlib zlibmodule.c -I./Modules/zlib -L./Modules/zlib -lz|' Modules/Setup && \
		make OPT="-fPIC -O2" && \
		make install
	
	rm -f libpython2.7.zip
	cd cpython/dist/lib/python2.7 && \
		LIBFILES=$$(find . '(' -regex './\(test\|idlelib\|lib2to3\|unittest\)' -o -regex '.*/tests*/.*' ')' -a -prune -o -name '*.pyo' -print) && \
		for f in $$LIBFILES; do zip $$OLDPWD/libpython2.7.zip $$f; done
	cd -

clean:
	cargo clean

prebuild:
	cargo build $(CARGO_ARGS) -p python27-sys
	rm -f target/*/pyinrs

dynamic: checkmode prebuild
	CMD=$$(cargo rustc $(FEAT) --bin pyinrs -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		tr ' ' '\n' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"

static: checkmode prebuild
	CMD=$$(cargo rustc $(FEAT) --bin pyinrs -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		tr ' ' '\n' | \
		grep -v '"\(-lpython2\.7\|-pie\|-Wl,.*-whole-archive\|-Wl,-B.*\)"' | \
		sed 's/"-lgcc_s"/"-lgcc_eh"/' | \
		sed 's/"cc"/"cc" "-static"/' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"
