PCFG = export PKG_CONFIG_PATH=$(shell pwd)/cpython/dist/lib/pkgconfig
MUSL_PCFG = export PKG_CONFIG_PATH=$(shell pwd)/cpython_musl/dist/lib/pkgconfig
export PKG_CONFIG_ALL_STATIC=1
# Having musl libc here would break build scripts
export LIBRARY_PATH=
# Musl is considered cross compiling
export PKG_CONFIG_ALLOW_CROSS=1

default:
	@echo "Choose one of 'prep', 'static', 'dynamic', 'clean'"

OPT ?= 0
MODE ?= wrap

checkmusl:
	@[ -f "$$(which musl-gcc)" ] || \
		(echo "Please add musl-gcc to your path" && exit 1)

CARGO_ARGS =
FEAT = --features "$(MODE)"
RUSTC_ARGS = --cfg 'feature="$(MODE)"'
ifeq ($(OPT),1)
	CARGO_ARGS += --release
	RUSTC_ARGS += -C opt-level=3
endif

WRAP_CMD = cat
ifeq ($(MODE),wrap)
	WRAP_SYMS = \
		read pread pread64 pwrite pwrite64 open open64 openat openat64 \
		creat creat64 lseek lseek64 \
		stat stat64 __xstat __xstat64 \
		lstat lstat64 __lxstat __lxstat64 \
		fstat fstat64 __fxstat __fxstat64 \
		access \
		chdir fchdir getcwd getwd get_current_dir_name \
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

prepmusl: checkmusl
	cd cpython_musl/Modules/zlib && \
		CC=musl-gcc CFLAGS="-fPIC" ./configure && \
		make libz.a
	
	cd cpython_musl && \
		./configure CC=musl-gcc LDFLAGS=-static --prefix=$$(pwd)/dist --disable-shared && \
		sed -i 's/^#define \(HAVE_GETC_UNLOCKED\).*/#undef \1/' pyconfig.h && \
		sed -i 's/^#\(array\|cmath\|math\|_struct\|time\|operator\|_random\|_collections\|_heapq\|itertools\|_functools\|datetime\|unicodedata\|_io\|fcntl\|select\|_socket\|termios\|resource\|_md5\|_sha\|_sha256\|_sha512\|binascii\|cStringIO\|cPickle\) /\1 /' Modules/Setup && \
		sed -i 's|^#zlib.*$$|zlib zlibmodule.c -I./Modules/zlib -L./Modules/zlib -lz|' Modules/Setup && \
		make OPT="-fPIC -O2" && \
		make install
	
	rm -f libpython2.7.zip
	cd cpython_musl/dist/lib/python2.7 && \
		LIBFILES=$$(find . '(' -regex './\(test\|idlelib\|lib2to3\|unittest\)' -o -regex '.*/tests*/.*' ')' -a -prune -o -name '*.pyo' -print) && \
		for f in $$LIBFILES; do zip $$OLDPWD/libpython2.7.zip $$f; done
	cd -

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
	cargo fetch
	rm -f target/**/pyinrs

dynamic: checkmode prebuild
	CMD=$$($(PCFG) && cargo rustc $(CARGO_ARGS) $(FEAT) --bin pyinrs -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		tr ' ' '\n' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"

static: checkmode prebuild
	CMD=$$($(PCFG) && cargo rustc $(CARGO_ARGS) $(FEAT) --bin pyinrs -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		sed 's/"-l" "python2\.7" //g' | \
		tr ' ' '\n' | \
		grep -v '"\(-pie\|-Wl,.*-whole-archive\|-Wl,-B.*\)"' | \
		sed 's/gcc_s"/gcc_eh"/' | \
		sed 's/"cc"/"cc" "-static"/' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"

musl: checkmode prebuild checkmusl
	CMD=$$($(MUSL_PCFG) && cargo rustc $(CARGO_ARGS) $(FEAT) --bin pyinrs --target x86_64-unknown-linux-musl -- $(RUSTC_ARGS) --emit obj -Z print-link-args | \
		tail -n 1 | \
		sed 's/"-l" "python2\.7" //g' | \
		tr ' ' '\n' | \
		$(WRAP_CMD) | \
		tr '\n' ' ') && \
		echo $$CMD && eval "$$CMD"
