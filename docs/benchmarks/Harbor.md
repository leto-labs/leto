# Harbor

## Overview

This page is the benchmark-research view of `Harbor`: why it matters as a
common harness, what it exposes about benchmark structure, and what the task
suite looks like.

For the operational runner, supported surfaces, prerequisites, current runbook,
and latest production-facing findings, see [docs/Harbor.md](/home/leovigna/Documents/projects/leovigna/mauser/docs/Harbor.md).

## Why Harbor Matters

Harbor is the most useful current harness for this repo because it gives us a
common execution layer across:

- built-in benchmark agents such as `codex`, `mini-swe-agent`, and `terminus-2`
- repo-local ACP-backed agents such as `codex-acp` and `brain-acp`

That makes it more valuable than a benchmark that effectively hardcodes one
agent runtime.

## Research Takeaways

The main benchmark-research conclusions so far are:

- Harbor is good enough to host real same-task comparisons across mature
  built-in agents and repo-local ACP surfaces
- the `terminal-bench-sample@2.0` ladder is useful for bounded loop iteration
- full `terminal-bench@2.0` runs are realistic enough to reveal loop failures
  that checkpoint tasks will miss
- the suite mixes task types heavily enough that resource limits, timeout
  budgets, and long-horizon tool use matter as much as one-shot correctness

## Terminal Bench 2.0 Matrix

The current full `terminal-bench@2.0` Harbor run expands to `89` tasks.

This matrix is derived from:

- the actual `89` trial directories created by the current full-suite Harbor job
- each task's cached Harbor [task.toml](/home/leovigna/.cache/harbor/tasks)

Machine limits come from each task's Harbor environment config. When a task
does not set them explicitly, Harbor defaults to `1 CPU`, `2048 MB` memory,
and `10 GB` storage.

| Task | Description | Difficulty | Type | Agent Timeout | Verifier Timeout | Machine Limits |
| --- | --- | --- | --- | --- | --- | --- |
| adaptive-rejection-sampler | Your task is to implement an adaptive-rejection sampler as described in Gilks et al. (1992). | medium | scientific-computing | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| bn-fit-modify | You are provided with a dataset sample from a Bayesian Network in /app/bn_sample_10k.csv. Your first task is to recov... | hard | scientific-computing | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| break-filter-js-from-html | There's a /app/filter.py script that attempts to remove JavaScript from HTML files to prevent XSS attacks. The /app/f... | medium | security | 1200 | 1200 | 1 CPU, 2G RAM, 10G disk |
| build-cython-ext | pyknotid is a Python package for computational topology and knot theory. I want to use its fast Cython extensions imp... | medium | debugging | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| build-pmars | Build pMARS (http://www.koth.org/pmars/) from source with no X server. | medium | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| build-pov-ray | Build POV-Ray 2.2. Find and download the source archives, extract them to /app/povray-2.2, then compile and install t... | medium | software-engineering | 12000 | 12000 | 1 CPU, 2G RAM, 10G disk |
| caffe-cifar-10 | Install the original BVLC Caffe deep learning framework (version 1.0.0) and train a convolutional neural network to c... | medium | machine-learning | 1200 | 1200 | 1 CPU, 2G RAM, 10G disk |
| cancel-async-tasks | Create a Python function called async run_tasks(tasks: list[Callable[[], Awaitable[None]]], max_concurrent: int) -> N... | hard | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| chess-best-move | The file chess_board.png has an image of a chess board. It is currently white to move. Write the best move for white... | medium | games | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| circuit-fibsqrt | The file /app/sim.c is a logic-gate simulator: each line of the file /app/gates.txt should contain a line of the form... | hard | software-engineering | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| cobol-modernization | You are given a COBOL program located at /app/src/program.cbl. This program reads input data from /app/src/INPUT.DAT... | easy | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| code-from-image | You're given an image of a pseudocode snippet at /app/code.png. Implement the intended logic of that snippet in any l... | medium | software-engineering | 1200 | 1200 | 1 CPU, 2G RAM, 10G disk |
| compile-compcert | Under /tmp/CompCert/, build the CompCert C verified compiler (version 3.13.1) from source, configuring it to the oper... | medium | system-administration | 2400 | 2400 | 2 CPU, 4G RAM, 10G disk |
| configure-git-webserver | Configure a git server so that I can run on my computer git clone user@server:/git/server echo "hello world" > hello.... | hard | system-administration | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| constraints-scheduling | Find a 1-hour meeting slot for Alice, Bob, and Carol during January 15-19, 2024 (business hours: 9 AM - 6 PM, Monday-... | medium | personal-assistant | 1200 | 1200 | 1 CPU, 2G RAM, 10G disk |
| count-dataset-tokens | Tell me how many deepseek tokens are there in the science domain of the ryanmarten/OpenThoughts-1k-sample dataset on... | medium | model-training | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| crack-7z-hash | You need to create a file called "/app/solution.txt" with the word found in "secret_file.txt" in the "secrets.7z" arc... | medium | security | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| custom-memory-heap-crash | I have a program at /app/ that crashes under RELEASE mode but not under DEBUG mode. Please help me fix it! You shall... | medium | debugging | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| db-wal-recovery | I have a database in WAL (Write-Ahead Logging) mode in /app/. However, the WAL file appears to be corrupted or encryp... | medium | file-operations | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| distribution-search | Finding a target probability distribution for LLM confidence metrics: | medium | machine-learning | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| dna-assembly | The file titled sequences.fasta contains the following sequences: * input: A circular input plasmid. * egfp: A linear... | hard | scientific-computing | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| dna-insert | The file titled sequences.fasta contains the sequence for a circular input plasmid, and a desired output plasmid. Des... | medium | scientific-computing | 1800 | 1800 | 1 CPU, 4G RAM, 10G disk |
| extract-elf | I have provided a file a.out that's a compiled C binary. Write me a program extract.js that, when run with node extra... | medium | file-operations | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| extract-moves-from-video | Download this video of someone playing zork. https://www.youtube.com/watch?v=ZCbvyPbhRfA. Then transcribe the entire... | hard | file-operations | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| feal-differential-cryptanalysis | The file /app/feal.py implements a FEAL-like encryption function. Implement a chosen plaintext attack that recovers t... | hard | mathematics | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| feal-linear-cryptanalysis | The file /app/feal.c implements a FEAL-like encryption function, with /app/decrypt.c being the corresponding decrypti... | hard | mathematics | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| filter-js-from-html | Create a python file /app/filter.py that removes JavaScript from HTML files to prevent XSS attacks. You should preser... | medium | security | 1800 | 900 | 1 CPU, 2G RAM, 10G disk |
| financial-document-processor | You have a collection of mixed document files in the /app/documents/ directory containing both JPG and PDF files. | medium | data-processing | 1200 | 1200 | 1 CPU, 4G RAM, 10G disk |
| fix-code-vulnerability | You need to identify and fix the vulnerability in the code according to the Common Weakness Enumeration (CWE). | hard | security | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| fix-git | I just made some changes to my personal site and checked out master, but now I can't find those changes. Please help... | easy | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| fix-ocaml-gc | I have been making some changes to the OCaml garbage collector, improving the sweeping performance by run-length comp... | hard | software-engineering | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| gcode-to-text | I have provided a file called text.gcode. This is a gcode file for a Prusa MK4s. I'm trying to print some text onto a... | medium | file-operations | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| git-leak-recovery | A secret was accidentally committed to this repository in /app and then removed by rewriting history. Please 1. recov... | medium | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| git-multibranch | Set up a Git server that hosts a project over SSH at git@localhost:/git/project. The server should accept password au... | medium | system-administration | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| gpt2-codegolf | I have downloaded the gpt-2 weights stored as a TF .ckpt. Write me a dependency-free C file that samples from the mod... | hard | software-engineering | 900 | 900 | 1 CPU, 4G RAM, 10G disk |
| headless-terminal | Implement the provided BaseTerminal interface. It provides a simple python interface to send keys to a headless termi... | medium | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| hf-model-inference | Set up a local service to run inference with a Hugging Face transformer model. | medium | data-science | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| install-windows-3.11 | Run Windows 3.11 for Workgroups in a virtual machine using qemu. You image is in /app/isos/win311.img. This image is... | hard | system-administration | 3600 | 3600 | 2 CPU, 4G RAM, 10G disk |
| kv-store-grpc | You need to build a simple KV store server using grpc that records number values for different string keys. Your serv... | medium | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| large-scale-text-editing | Transform /app/input.csv (1 million rows) to match /app/expected.csv exactly using **keystroke-efficient** Vim macros... | medium | file-operations | 1200 | 1200 | 1 CPU, 2G RAM, 10G disk |
| largest-eigenval | Complete the implementation in find_dominant_eigenvalue_and_eigenvector in /app/eigen.py. "Dominant" means the eigenv... | medium | mathematics | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| llm-inference-batching-scheduler | In this task you will implement an **LLM inference batching scheduler (shape‑aware)** for a static graph LLM inferenc... | hard | machine-learning | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| log-summary-date-ranges | You are given multiple log files stored in /app/logs. Each log file name follows the pattern YYYY-MM-DD_<source>.log... | medium | data-processing | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| mailman | Spin up a mailing list server for our reading group: reading-group@local.edu using postfix and mailman3 (both are ins... | medium | system-administration | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| make-doom-for-mips | I have provided /app/doomgeneric/, the source code to doom. I've also wrote a special doomgeneric_img.c that I want y... | hard | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| make-mips-interpreter | I have provided /app/doomgeneric_mips, a MIPS elf file, along with doomgeneric/, the corresponding source code. Pleas... | hard | software-engineering | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| mcmc-sampling-stan | Sample from a hierarchical Bayesian model using R and Stan, and estimate the posterior means of the parameters. | hard | data-science | 1800 | 1800 | 4 CPU, 8G RAM, 10G disk |
| merge-diff-arc-agi-task | mkdir /app/repo, then initialize a git repo at /app/repo. Fetch the first git bundle located at /app/bundle1.bundle a... | medium | debugging | 900 | 900 | 1 CPU, 4G RAM, 10G disk |
| model-extraction-relu-logits | I have a ReLU neural network that you can query by importing forward.py and calling forward(x) which returns a single... | hard | mathematics | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| modernize-scientific-stack | Modernize Legacy Scientific Computing Stack | medium | scientific-computing | 600 | 600 | 1 CPU, 2G RAM, 10G disk |
| mteb-leaderboard | I have a lot of Scandinavian texts that I need to encode. Please provide me with the name in organization/model_name... | medium | data-science | 3600 | 3600 | 1 CPU, 4G RAM, 10G disk |
| mteb-retrieve | Given the query "terminal-bench" retrieve the document with the 5th highest cosine similarity among the texts at /app... | medium | data-science | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| multi-source-data-merger | Merge user data from three different sources with different formats and schemas. | medium | data-processing | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| nginx-request-logging | Set up an Nginx web server with advanced request logging and custom configurations. Your task is to: | medium | system-administration | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| openssl-selfsigned-cert | Your company needs a self-signed TLS certificate for an internal development server. Create a self-signed certificate... | medium | security | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| overfull-hbox | Ensure that the LaTeX document main.tex compiles successfully using the installed TeX distribution and pdflatex compi... | easy | debugging | 750 | 360 | 2 CPU, 4G RAM, 10G disk |
| password-recovery | You need to perform a digital forensic recovery task. A system administrator has accidentally deleted an important fi... | hard | security | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| path-tracing | I've put an image at /app/image.ppm that I rendered programmatically. Write a c program image.c that I can run and co... | hard | software-engineering | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| path-tracing-reverse | I've compiled a program at /app/mystery. Write a C program /app/mystery.c that performs an identical operation. You c... | hard | software-engineering | 1800 | 1800 | 1 CPU, 2G RAM, 10G disk |
| polyglot-c-py | Write me a single file in /app/polyglot/main.py.c which is a polyglot, that is, I can run | medium | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| polyglot-rust-c | Write me a single file in /app/polyglot/main.rs which is a polyglot, that is, I can run | hard | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| portfolio-optimization | You have a Python baseline in portfolio_baseline.py that calculates portfolio risk and return using nested loops. Com... | medium | optimization | 3600 | 3600 | 1 CPU, 4G RAM, 10G disk |
| protein-assembly | I am planning an experiment where I'll be testing the stability of dihydrofolate reductase (DHFR) with FRET. I have a... | hard | scientific-computing | 1800 | 1800 | 1 CPU, 4G RAM, 10G disk |
| prove-plus-comm | Fix the incomplete proof of addition commutativity in the file plus_comm.v. The file contains a partial proof that ne... | easy | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| pypi-server | Create a python package called vectorops, then build this package and set up a pypi server on port 8080 locally which... | medium | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| pytorch-model-cli | Your task is to implement a command line tool that can be used to run inference on an MNIST model. The tool should be... | medium | model-training | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| pytorch-model-recovery | - You are given a PyTorch state dictionary (/app/weights.pt) representing the weights of a Pytorch model, and a datas... | medium | model-training | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| qemu-alpine-ssh | Start the /app/alpine.iso image in qemu, and then start an ssh server on the alpine linux vm. When you're done, I sho... | medium | system-administration | 900 | 900 | 1 CPU, 4G RAM, 10G disk |
| qemu-startup | Start the /app/alpine.iso image in qemu in such a way that I can connect to it via telnet 127.0.0.1 6665. When I run... | medium | system-administration | 900 | 900 | 1 CPU, 4G RAM, 10G disk |
| query-optimize | You are given the Open English Wordnet (OEWN) database in SQLite format, located at /app/oewn.sqlite. | medium | data-science | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| raman-fitting | You are given the output file of a Raman Setup. We used it to measure some graphene sample. Fit the G and 2D Peak of... | medium | scientific-computing | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| regex-chess | Write a JSON file called /app/re.json that is a list of [regex, replacement] pairs. When executed in order with the P... | hard | software-engineering | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| regex-log | Write a regex expression that matches dates in the format YYYY-MM-DD appearing in lines that contain an IPv4 address... | medium | data-processing | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| reshard-c4-data | Help me create two scripts for managing the resharding of my dataset: | medium | data-science | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| rstan-to-pystan | You are given datasets /app/train_X.csv, /app/train_y.csv, /app/test_X.csv, /app/meta_public.json; and a R script /ap... | medium | data-science | 1800 | 1800 | 4 CPU, 8G RAM, 10G disk |
| sam-cell-seg | I have annotated histopathology slides with cell masks. The problem is that some of the masks are rectangles, while t... | hard | data-science | 7200 | 7200 | 1 CPU, 4G RAM, 10G disk |
| sanitize-git-repo | Please help sanitize my github repository "dclm" of all API keys. | medium | security | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| schemelike-metacircular-eval | Write a file eval.scm that is a metacircular evaluator capable of interpreting the scheme-like language implemented b... | medium | software-engineering | 2400 | 2400 | 1 CPU, 2G RAM, 10G disk |
| sparql-university | You are provided with a Turtle file at /app/university_graph.ttl containing both the ontology and the instance data f... | hard | data-querying | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| sqlite-db-truncate | I have a sqlite database in /app/trunc.db that was corrupted through binary truncation. Recover as many of the rows a... | medium | debugging | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| sqlite-with-gcov | Compile SQLite in /app/sqlite with gcov instrumentation and make it available in the PATH. The SQLite source snapshot... | medium | system-administration | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| torch-pipeline-parallelism | Implement pipeline parallel training for the LLaMA model using PyTorch. Create the file /app/pipeline_parallel.py and... | hard | software-engineering | 900 | 900 | 1 CPU, 4G RAM, 10G disk |
| torch-tensor-parallelism | Implement tensor parallelism for linear layers using PyTorch. Create the file /app/parallel_linear.py and implement t... | hard | software-engineering | 900 | 900 | 1 CPU, 4G RAM, 10G disk |
| train-fasttext | Please train a fasttext model on the yelp data in the data/ folder. | hard | model-training | 3600 | 3600 | 1 CPU, 4G RAM, 10G disk |
| tune-mjcf | Can you tune this MuJoCo model file (mjcf) such that it takes 60% of the original time or less to simulate the same s... | medium | scientific-computing | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| video-processing | Write a script, named jump_analyzer.py, and place it in /app/jump_analyzer.py . The script analyzes MP4 videos of hur... | hard | video-processing | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| vulnerable-secret | Analyze the following program that contains a secret key. The goal is to extract this secret key and save it to /app/... | medium | security | 900 | 900 | 1 CPU, 2G RAM, 10G disk |
| winning-avg-corewars | Write a CoreWars program that achieves a winning average against five classic opponent warriors. pMARS (CoreWars simu... | medium | software-engineering | 3600 | 3600 | 1 CPU, 2G RAM, 10G disk |
| write-compressor | I have a decompressor in /app/decomp.c. It reads compressed data from stdin and writes the decompressed data to stdou... | hard | software-engineering | 900 | 900 | 1 CPU, 2G RAM, 10G disk |

## Running Benchmarks

Examples:

```bash
./scripts/harbor-run.sh codex hello-world@1.0
./scripts/harbor-run.sh terminus-2 terminal-bench-sample@2.0 chess-best-move
just harbor-run codex hello-world@1.0
just harbor-run mini-swe-agent terminal-bench-sample@2.0 regex-log
just harbor-run codex-acp terminal-bench-sample@2.0 chess-best-move
HARBOR_BRAIN_LOOP=robust just harbor-run brain-acp terminal-bench-sample@2.0 regex-log
just harbor-run brain-acp terminal-bench@2.0
just harbor-run codex terminal-bench-sample@2.0 sqlite-with-gcov
```

Useful discovery command:

```bash
just harbor-datasets
```

## Agent Notes

### Built-in `codex`

- uses Harbor's built-in `codex` integration
- requires `OPENAI_API_KEY`
- current stable built-in Harbor baseline

### Built-in `mini-swe-agent`

- uses Harbor's built-in `mini-swe-agent` integration
- accepts `MSWEA_API_KEY` or provider-specific auth such as `OPENAI_API_KEY`
- supported as a benchmark surface, but currently weaker on heavier environment-repair tasks like `sqlite-with-gcov`

### Built-in `terminus-2`

- uses Harbor's built-in `terminus-2` integration
- is registered as a native Harbor agent in repocache:
  [name.py](/home/leovigna/Documents/projects/leovigna/mauser/repocache/harbor-framework/harbor/src/harbor/models/agent/name.py),
  [factory.py](/home/leovigna/Documents/projects/leovigna/mauser/repocache/harbor-framework/harbor/src/harbor/agents/factory.py)
- serves as a fifth reference surface because it ships with Harbor itself

### Repo-local `codex-acp`

- Harbor agent import path:
  `tools.harbor.agents.acp_codex:AcpCodexAgent`
- uses the shared Harbor ACP client and shared `BaseAcpAgent`
- copies a local `codex-acp` binary into the Harbor task environment
- runs `codex-acp` inside the container over ACP stdio
- prefers host Codex auth from `~/.codex/auth.json`
- supports `HARBOR_AUTH_METHOD=openai-api-key` as fallback

### Repo-local `brain-acp`

- Harbor agent import path:
  `tools.harbor.agents.acp_brain:AcpBrainAgent`
- uses the shared Harbor ACP client and shared `BaseAcpAgent`
- prefers a portable Harbor artifact at
  `target/x86_64-unknown-linux-musl/release/brain-acp`
- falls back to host-local `target/release/brain-acp` or `target/debug/brain-acp`
- copies the selected local `brain-acp` binary into the Harbor task environment
- stages `~/.brain/credentials` and optional `config.toml` into a fresh container `BRAIN_HOME`
- supports `HARBOR_BRAIN_LOOP=robust` for stronger bounded-task behavior

To build the preferred Harbor artifact, use the repo musl build path:

```bash
just build-release
```

This requires a musl C toolchain such as `musl-gcc` to be installed on the host.
