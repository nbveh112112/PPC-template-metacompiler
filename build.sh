set -e

METACOMPILER="/home/timur/diploma/template_metacompiler/target/debug/PPC_template_metacompiler"

if [ $# -lt 1 ]; then
    echo "Usage: $0 <source-file> [output-executable]"
    exit 1
fi

SOURCE="$1"
OUTPUT_EXE="${2:-a.out}"

if [ ! -f "$SOURCE" ]; then
    echo "Error: source file '$SOURCE' not found."
    exit 1
fi

if [ ! -x "$METACOMPILER" ]; then
    echo "Error: metacompiler '$METACOMPILER' not found or not executable."
    exit 1
fi

PREPROCESSED="${SOURCE%.*}.i"
META_OUTPUT="${SOURCE%.*}.meta.c"

echo "Step 1: Preprocessing $SOURCE -> $PREPROCESSED"
clang -E -x c "$SOURCE" -o "$PREPROCESSED"

echo "Step 2: Metacompiling $PREPROCESSED -> $META_OUTPUT"
"$METACOMPILER" --input "$PREPROCESSED" --output "$META_OUTPUT"

echo "Step 3: Compiling $META_OUTPUT -> $OUTPUT_EXE"
clang "$META_OUTPUT" -o "$OUTPUT_EXE"

echo "Done. Executable created: $OUTPUT_EXE"
# rm -f "$PREPROCESSED" "$META_OUTPUT"