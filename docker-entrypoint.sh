#!/bin/bash
set -e

# Source Intel OneAPI if available
if [ -f /opt/intel/oneapi/setvars.sh ]; then
    source /opt/intel/oneapi/setvars.sh > /dev/null 2>&1 || true
fi

# Dispatch based on command
case "$1" in
    execute)
        shift
        if [ "$1" = "-a" ] || [ "$1" = "--airbender" ]; then
            shift
            exec ./execute-airbender.sh "$@"
        else
            exec ./execute-zisk.sh "$@"
        fi
        ;;
    prove)
        shift
        # Check for proving key
        if [ ! -d "/workspace/provingKey" ]; then
            echo "Error: provingKey directory not mounted."
            echo "Mount it with: -v /path/to/provingKey:/workspace/provingKey:ro"
            exit 1
        fi
        exec ./prove-block-zisk.sh "$@"
        ;;
    shell)
        exec /bin/bash
        ;;
    -h|--help|help|"")
        echo "ZisK/ZKsyncOS Docker Container"
        echo ""
        echo "Usage: docker run [options] zisk-os <command>"
        echo ""
        echo "Commands:"
        echo "  execute            Execute Ethereum block with ZisK emulator"
        echo "  execute -a         Execute Ethereum block with Airbender"
        echo "  prove              Run GPU proving (requires mounted provingKey)"
        echo "  shell              Interactive shell in container"
        echo ""
        echo "Environment Variables:"
        echo "  BLOCK=<number>     Block number to execute/prove (default: 24198369)"
        echo ""
        echo "Examples:"
        echo "  docker run --rm zisk-os execute"
        echo "  docker run --rm -e BLOCK=24198369 zisk-os execute"
        echo "  docker run --rm --gpus all -v ./provingKey:/workspace/provingKey:ro zisk-os prove"
        echo "  docker run --rm -it zisk-os shell"
        exit 0
        ;;
    *)
        echo "Unknown command: $1"
        echo "Run with --help for usage information."
        exit 1
        ;;
esac
