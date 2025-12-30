#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit

mkdir out/ || true
rm -r pkg/ || true
mkdir pkg/

if [ "${MINIMAL:-0}" = "1" ]; then
	CARGOFLAGS="--no-default-features"
else
	CARGOFLAGS=""
fi

WBG="wasm-bindgen 0.2.106"
if [ "$(wasm-bindgen -V)" != "$WBG" ]; then
	echo "Incorrect wasm-bindgen version: '$(wasm-bindgen -V)' != '$WBG'"
	exit 1
fi

export CFLAGS='-O3' RUSTFLAGS='-Zlocation-detail=none'
cargo build --target wasm32-unknown-unknown -Z build-std=panic_abort,std -Z build-std-features=optimize_for_size --release $CARGOFLAGS "$@"
echo "[wbg] cargo finished"
wasm-bindgen --target web --out-dir out/ target/wasm32-unknown-unknown/release/async_brotli_wasm.wasm
echo "[wbg] wasm-bindgen finished"

if ! [ "${RELEASE:-0}" = "1" ]; then
	: "${WASMOPTFLAGS:=-g}"
else
	: "${WASMOPTFLAGS:=}"
fi

mv out/async_brotli_wasm_bg.wasm out/async_brotli_wasm_unoptimized.wasm

if [ "${RELEASE:-0}" = "1" ]; then
	(
		G="--generate-global-effects"
		# shellcheck disable=SC2086
		time wasm-opt $WASMOPTFLAGS \
			out/async_brotli_wasm_unoptimized.wasm -o out/async_brotli_wasm_bg.wasm \
			--converge \
			$G --type-unfinalizing $G --type-ssa $G -O4 $G --flatten $G --rereloop $G -O4 $G -O4 $G --type-merging $G --type-finalizing $G -O4 \
			$G --type-unfinalizing $G --type-ssa $G -Oz $G --flatten $G --rereloop $G -Oz $G -Oz $G --type-merging $G --type-finalizing $G -Oz \
			$G --abstract-type-refining $G --code-folding $G --const-hoisting $G --dae $G --flatten $G --dfo $G --merge-locals $G --merge-similar-functions --type-finalizing \
			$G --type-unfinalizing $G --type-ssa $G -O4 $G --flatten $G --rereloop $G -O4 $G -O4 $G --type-merging $G --type-finalizing $G -O4 \
			$G --type-unfinalizing $G --type-ssa $G -Oz $G --flatten $G --rereloop $G -Oz $G -Oz $G --type-merging $G --type-finalizing $G -Oz 
	)
else
	mv out/async_brotli_wasm_unoptimized.wasm out/async_brotli_wasm_bg.wasm
fi
echo "[wbg] wasm-opt finished"

cp out/async_brotli_wasm.{js,d.ts} pkg/
cp out/async_brotli_wasm_bg.wasm pkg/async_brotli_wasm.wasm

rm -r out/
echo "[wbg] done!"
