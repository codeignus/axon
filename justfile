# Axon self-hosting workflow
# Build with the installed axon, then exercise the freshly built binary.

bindir := "target/build/axon"

# Default: build and run full self-hosting verify
default: test

# First-stage build only (installed axon → fresh binary)
prebuild:
    axon build

# Full self-hosting build: prebuild, then rebuild with the new binary
build: prebuild
    {{bindir}}/axon build

# Prebuild, then run check with the freshly built binary
check: prebuild
    {{bindir}}/axon check

# Prebuild, then run test with the freshly built binary
test: prebuild
    {{bindir}}/axon test

# Build twice (self-hosting handshake: old builds new, new builds newer)
self-host: build
    {{bindir}}/axon build

# Prebuild, then run fmt with the freshly built binary
fmt: prebuild
    {{bindir}}/axon fmt

# Prebuild, then run with the freshly built binary
run: prebuild
    {{bindir}}/axon run

# Clean build artifacts
clean:
    rm -rf target/build target/cache
