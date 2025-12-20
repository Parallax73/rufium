#!/bin/bas:h
set -e

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)     PLATFORM="linux" ;;
    Darwin)    PLATFORM="mac" ;;
    CYGWIN*|MINGW32*|MSYS*|MINGW*) PLATFORM="win" ;;
    *)         echo "Unknown OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64)    CPU="x64" ;;
    arm64|aarch64) CPU="arm64" ;;
    *)         echo "Unknown CPU: $ARCH"; exit 1 ;;
esac

echo "Detected: $PLATFORM - $CPU"

FILENAME="pdfium-$PLATFORM-$CPU.tgz"
DOWNLOAD_URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$FILENAME"

echo "Downloading $FILENAME..."
curl -L -o "$FILENAME" "$DOWNLOAD_URL"

echo "Extracting..."
tar -xzf "$FILENAME"

echo "Installing library..."
if [ "$PLATFORM" == "linux" ]; then
    cp lib/libpdfium.so .
    echo "libpdfium.so placed in project root."
elif [ "$PLATFORM" == "mac" ]; then
    cp lib/libpdfium.dylib .
    echo " libpdfium.dylib placed in project root."
elif [ "$PLATFORM" == "win" ]; then
    cp bin/pdfium.dll .
    echo "pdfium.dll placed in project root."
fi

rm "$FILENAME"
rm -rf lib bin include args.gn
echo "Installation complete"
