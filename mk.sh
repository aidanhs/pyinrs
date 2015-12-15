set -e
[ -d shutit ] || git clone https://github.com/ianmiell/shutit.git
rm -rf include
cp -r shutit include
cd include
    pip install -t dep -r requirements.txt --no-compile
    rm -rf .git
    rm -rf docs
    rm -rf test
    rm -rf examples
    rm -rf keyrings
    rm LICENSE .pylintrc .gitignore requirements.txt run_shutit_server.sh shutit
    cd dep
        rm -rf *.dist-info *.egg-info
    cd ..
    find_no_context() {
        local loc="$1"
        shift
        find "$loc" '!' '(' -type d -name context -prune ')' $@
    }
    find_no_context library -type d -name bin -prune -exec rm -r '{}' ';'
    find_no_context . -type f -name '*.md' -exec rm '{}' ';'
    find_no_context . -type f -name 'Dockerfile' -exec rm '{}' ';'
    find_no_context . -type f -name 'STOPTEST' -exec rm '{}' ';'
cd ..
cp libpython2.7.zip include
