# crucible: build on a CUDA machine. The card says what to build for, so
# one Makefile serves the Pascal cards (calber, boburner, whitewolf: sm_61,
# CUDA 12.9) and a newer one (saskatchewan's RTX 4060 Ti: Ada, sm_89, CUDA
# 13 on Ubuntu 26.04) with nothing set by hand.
#
#   make                 the binary for this machine's card, a net of H=32
#   make CAP=89          for a named architecture (a build without the card)
#   make H=48            a wider net (two nets must fit 48 KB of shared memory)
#   make which           what this machine would build, and with what
#   make run             100 generations, the numbers to runs/
NVCC ?= /usr/local/cuda/bin/nvcc
# The first card's compute capability, 6.1 read as 61, asked once; 61 if
# nvidia-smi cannot say (the driver's own query, there since driver 510).
ifndef CAP
CAP := $(or $(shell nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null | head -1 | tr -d ' .'),61)
endif
ARCH ?= -gencode arch=compute_$(CAP),code=sm_$(CAP)
# The toolkit's major version. CUDA 13 builds nothing older than sm_75, so a
# Pascal card keeps 12.x: said here, not as nvcc's unsupported-architecture
# error.
CUDA_MAJOR := $(shell $(NVCC) --version 2>/dev/null | sed -n 's/.*release \([0-9]*\)\..*/\1/p')
H    ?= 32
MINB ?= 2
FLAGS = -O3 -use_fast_math -std=c++17 -Wno-deprecated-gpu-targets -lineinfo

crucible: src/crucible.cu
	@if [ -n "$(CUDA_MAJOR)" ] && [ "$(CUDA_MAJOR)" -ge 13 ] && [ "$(CAP)" -lt 75 ]; then \
		echo "crucible: CUDA $(CUDA_MAJOR) cannot build for sm_$(CAP): keep CUDA 12 on this card" >&2; exit 1; fi
	$(NVCC) $(FLAGS) $(ARCH) -DHID=$(H) -DMINB=$(MINB) -Xptxas -v -o $@ $< 2>&1 | grep -E 'error|registers|spill' || true
	@test -x $@

which:
	@echo "card sm_$(CAP), CUDA $(or $(CUDA_MAJOR),?) ($(NVCC)), H=$(H), MINB=$(MINB)"

run: crucible
	mkdir -p runs
	./crucible --gens 100 --out runs/$$(hostname)-$$(date +%Y%m%d-%H%M%S).tsv

clean:
	rm -f crucible

.PHONY: run clean which
