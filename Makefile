# crucible: build on a CUDA machine. The card says what to build for, so
# one Makefile serves the Pascal cards (calber, boburner, whitewolf: sm_61,
# CUDA 12.9) and a newer one (saskatchewan's RTX 4060 Ti: Ada, sm_89, CUDA
# 13 on Ubuntu 26.04) with nothing set by hand.
#
#   make                 the binary for this machine's card, a net of H=32
#   make CAP=89          for a named architecture (a build without the card)
#   make HYBRID=1        the net overrides a scripted tank, as treadcore's transform does
#   make WORLD=1         treadcore's rules: its tank, its gun, 24 shells, eyes of 10 tiles,
#                        four neutral pillboxes; 20 senses more (52 in, 55 with HYBRID=1)
#   make WORLD=1 R_VMAX=4.5f R_PB_RANGE=9.0f   any of the game's numbers given, by
#                        crucible's name (the coordinator passes all of them, read from
#                        treadcore's rules.rs at the fleet's commit)
#   make H=48            a wider net (two nets must fit 48 KB of shared memory: the
#                        source refuses to compile otherwise; WORLD=1 HYBRID=1 H=48 is
#                        41,888 bytes, H=64 would be 64,032)
#
# A state or champion of a smaller net loads into a larger build widened,
# every new weight zero, so it plays as it did: a hybrid island's state goes
# on as WORLD=1 HYBRID=1 H=48 with --load, no generation lost.
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
# The game's numbers, a make variable each, so no quoting stands between the
# coordinator and the compiler: R_VMAX=4.5f is -DVMAX=4.5f (src/crucible.cu
# names which; WORLD=1 only).
RULES += $(foreach v,$(filter R_%,$(.VARIABLES)),-D$(patsubst R_%,%,$(v))=$($(v)))

crucible: src/crucible.cu
	@if [ -n "$(CUDA_MAJOR)" ] && [ "$(CUDA_MAJOR)" -ge 13 ] && [ "$(CAP)" -lt 75 ]; then \
		echo "crucible: CUDA $(CUDA_MAJOR) cannot build for sm_$(CAP): keep CUDA 12 on this card" >&2; exit 1; fi
	$(NVCC) $(FLAGS) $(ARCH) -DHID=$(H) -DMINB=$(MINB) $(if $(HYBRID),-DHYBRID) $(if $(WORLD),-DWORLD) $(RULES) -Xptxas -v -o $@ $< 2>&1 | grep -E 'error|registers|spill' || true
	@test -x $@

which:
	@echo "card sm_$(CAP), CUDA $(or $(CUDA_MAJOR),?) ($(NVCC)), H=$(H), MINB=$(MINB)$(if $(HYBRID), HYBRID)$(if $(WORLD), WORLD)$(if $(RULES), $(RULES))"

run: crucible
	mkdir -p runs
	./crucible --gens 100 --out runs/$$(hostname)-$$(date +%Y%m%d-%H%M%S).tsv

clean:
	rm -f crucible

.PHONY: run clean which
