# crucible: build on a CUDA machine (calber, boburner, whitewolf: CUDA 12.9,
# Pascal, sm_61). CUDA 13 drops Pascal: keep 12.x on these cards.
#
#   make                 the binary, a net of H=32
#   make H=48            a wider net (two nets must fit 48 KB of shared memory)
#   make run             100 generations, the numbers to runs/
NVCC ?= /usr/local/cuda/bin/nvcc
ARCH ?= -gencode arch=compute_61,code=sm_61
H    ?= 32
MINB ?= 2
FLAGS = -O3 -use_fast_math -std=c++17 -Wno-deprecated-gpu-targets -lineinfo

crucible: src/crucible.cu
	$(NVCC) $(FLAGS) $(ARCH) -DHID=$(H) -DMINB=$(MINB) -Xptxas -v -o $@ $< 2>&1 | grep -E 'error|registers|spill' || true
	@test -x $@

run: crucible
	mkdir -p runs
	./crucible --gens 100 --out runs/$$(hostname)-$$(date +%Y%m%d-%H%M%S).tsv

clean:
	rm -f crucible

.PHONY: run clean
