# Maintainer: Alastair Daivis <alaroldai@gmail.com>
pkgname=x11-tile
pkgver=0.1.0
pkgrel=1
makedepends=('rust' 'cargo')
depends=('libxcb' 'gcc-libs')
arch=('i686' 'x86_64' 'armv6h' 'armv7h')

build() {
    cargo build --release --locked --all-features
}

package() {
    install -Dm755 "../target/release/x11-tile" "$pkgdir/usr/bin/x11-tile"
}
