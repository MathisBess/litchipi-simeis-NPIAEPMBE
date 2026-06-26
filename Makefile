export RUSTFLAGS=-C code-model=kernel -C codegen-units=1

.PHONY: all build doc check test clean

all: build

build:
	@echo "Build du projet"
	cargo build --release --verbose
	@echo "Strip du binaire"
	strip target/release/simeis-server

doc:
	@echo "Creation de la doc"
	typst compile doc/manual.typ doc/manual.pdf

check:
	@echo "Vérifier le formatage"
	cargo fmt -- --check
	@echo "Vérifier la compilation"
	cargo check
	@echo "Linter le code"
	cargo clippy -- -D warnings -A clippy::clone_on_copy

test:
	@echo "Tests"
	cargo test

audit:
	@echo "Installation de cargo-audit"
	@command -v cargo-audit >/dev/null || cargo install cargo-audit
	@echo "Lancement de l'audit"
	cargo audit

clean:
	@echo "Clean"
	cargo clean
	rm -f doc/manual.pdf

debian: build
	@echo "Préparation du paquet Debian"
	mkdir -p pkg-debian/DEBIAN
	mkdir -p pkg-debian/usr/bin
	mkdir -p pkg-debian/usr/share/man/man1
	mkdir -p pkg-debian/etc/systemd/system
	mkdir -p pkg-debian/usr/share/doc/simeis-npiaepmbe

	cp debian-meta/control pkg-debian/DEBIAN/
	cp debian-meta/postinst pkg-debian/DEBIAN/
	cp debian-meta/simeis.service pkg-debian/etc/systemd/system/
	cp debian-meta/simeis.1 pkg-debian/usr/share/man/man1/
	cp debian-meta/copyright pkg-debian/usr/share/doc/simeis-npiaepmbe/
	cp debian-meta/changelog pkg-debian/usr/share/doc/simeis-npiaepmbe/
	cp debian-meta/changelog.Debian pkg-debian/usr/share/doc/simeis-npiaepmbe/

	gzip --best pkg-debian/usr/share/man/man1/simeis.1
	gzip --best pkg-debian/usr/share/doc/simeis-npiaepmbe/changelog
	gzip --best pkg-debian/usr/share/doc/simeis-npiaepmbe/changelog.Debian

	cp target/release/simeis-server pkg-debian/usr/bin/

	chmod 755 pkg-debian/DEBIAN/postinst
	chmod 644 pkg-debian/etc/systemd/system/simeis.service
	chmod 644 pkg-debian/usr/share/man/man1/simeis.1.gz
	chmod 644 pkg-debian/usr/share/doc/simeis-npiaepmbe/copyright
	chmod 644 pkg-debian/usr/share/doc/simeis-npiaepmbe/changelog.gz
	chmod 644 pkg-debian/usr/share/doc/simeis-npiaepmbe/changelog.Debian.gz

	dpkg-deb --build pkg-debian simeis.deb

	rm -rf pkg-debian
