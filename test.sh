set -e

. ./build_settings.sh
export PROJ_DIR=${PWD}

if [ ! -d build ]; then
    git submodule update --init --recursive
    export GTEST_DIR="${PWD}/gtest/googletest"
    cd ${GTEST_DIR} && mkdir build && cd build && cmake ${GTEST_DIR} && make
    cd ${PROJ_DIR} && mkdir build && cd build && mkdir gtest
    mv ${GTEST_DIR}/build/*.a gtest/
    cd ${PROJ_DIR} && premake4 gmake
    rm -rf "${GTEST_DIR}/build"
fi

cd "${PROJ_DIR}/build"
make test && ./test/test_program

