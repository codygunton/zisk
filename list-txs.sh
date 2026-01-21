#!/bin/bash
# Lists transactions from the ZisK execution log
# Usage: ./list-txs.sh [log_file] [--revert|--no-revert] [-s|--summary]

LOG_FILE="/tmp/zisk-execute.log"
SUMMARY=0
FILTER="" # "", "revert", or "no-revert"

for arg in "$@"; do
    case $arg in
        -s | --summary)
            SUMMARY=1
            ;;
        --revert)
            FILTER="revert"
            ;;
        --no-revert)
            FILTER="no-revert"
            ;;
        *)
            if [[ -f "$arg" ]]; then
                LOG_FILE="$arg"
            fi
            ;;
    esac
done

if [[ ! -f "$LOG_FILE" ]]; then
    echo "Error: Log file not found: $LOG_FILE" >&2
    exit 1
fi

# Extract all transactions and their revert status
awk '
/NEXT_TX_SIZE tx=/ {
    # Output previous transaction if exists
    if (current_tx != "") {
        print current_tx, reverted
    }
    # Extract transaction number
    match($0, /tx=([0-9]+)/, arr)
    current_tx = arr[1]
    reverted = 0
}
/revert = true/ {
    reverted = 1
}
END {
    # Handle the last transaction
    if (current_tx != "") {
        print current_tx, reverted
    }
}
' "$LOG_FILE" | sort -t' ' -k1 -n | uniq > /tmp/all-txs.tmp

# Filter based on user selection
case $FILTER in
    revert)
        awk '$2 == 1 { print $1 }' /tmp/all-txs.tmp > /tmp/filtered-txs.tmp
        ;;
    no-revert)
        awk '$2 == 0 { print $1 }' /tmp/all-txs.tmp > /tmp/filtered-txs.tmp
        ;;
    *)
        awk '{ print $1 }' /tmp/all-txs.tmp > /tmp/filtered-txs.tmp
        ;;
esac

if [[ $SUMMARY -eq 1 ]]; then
    total=$(wc -l < /tmp/all-txs.tmp)
    filtered=$(wc -l < /tmp/filtered-txs.tmp)
    case $FILTER in
        revert)
            echo "Reverting transactions: $filtered / $total"
            ;;
        no-revert)
            echo "Non-reverting transactions: $filtered / $total"
            ;;
        *)
            echo "Total transactions: $total"
            ;;
    esac
else
    cat /tmp/filtered-txs.tmp
fi

rm -f /tmp/all-txs.tmp /tmp/filtered-txs.tmp
