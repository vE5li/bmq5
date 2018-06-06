# rust
RTARGET 		:= target/debug/bmq5-server
FEAT 			?= controller driver system

# files
OUTPUT 			?= /usr/bin/bmq5-server

# build
all: build

# nothing
clean:
	cargo clean

# compile
build:
	cargo build --features "$(FEAT)"

# build without features
minimal:
	cargo build

# copy
copy:
	cp $(RTARGET) $(OUTPUT)
