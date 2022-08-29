# DB location values
DB_FILE_NAME=rand_db.json
DB_FILE_PATH=data/${DB_FILE_NAME}
PARAMS_OUTPUT_PATH=data/params.json
PREVIOUS_DIR=..

# cargo build values
MATRIX_HEIGHT_EXP=16
LWE_DIMENSION=1572
ELEMENT_SIZE_EXP=13
PLAINTEXT_SIZE_EXP=10
NUM_SHARDS=8

# rust flags 
RUST_BACKTRACE=1

# python db generation values
DB_ALL_ONES=0
DB_NUM_ENTRIES_EXP=${MATRIX_HEIGHT_EXP}

# local environment values
LOCAL_TEST_DATA=data/local-env
LOCAL_BUCKETS_PATH=${LOCAL_TEST_DATA}/local_buckets
LOCAL_CONFIGS=${LOCAL_TEST_DATA}/local-configs.yml
START_CONTAINERS_SCRIPT_PATH=${LOCAL_TEST_DATA}/start-server-containers.sh
ROOT_DIR:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))

RUST_FLAGS=RUST_BACKTRACE=${RUST_BACKTRACE}
DB_ENV=DB_FILE=${PREVIOUS_DIR}/${DB_FILE_PATH} PARAMS_OUTPUT_PATH=${PREVIOUS_DIR}/${PARAMS_OUTPUT_PATH}
PRELIM=${RUST_FLAGS} ${DB_ENV}
PIR_FLAGS=-m ${MATRIX_HEIGHT_EXP} --dim ${LWE_DIMENSION} --ele_size ${ELEMENT_SIZE_EXP} --plaintext_bits ${PLAINTEXT_SIZE_EXP} --num_shards ${NUM_SHARDS}
PIR_ENV=PIR_MATRIX_HEIGHT_EXP=${MATRIX_HEIGHT_EXP} PIR_LWE_DIM=${LWE_DIMENSION} PIR_ELE_SIZE_EXP=${ELEMENT_SIZE_EXP} PIR_PLAINTEXT_BITS=${PLAINTEXT_SIZE_EXP} PIR_NUM_SHARDS=${NUM_SHARDS}
PIR_ENV_ALL=PIR_LWE_DIM=${LWE_DIMENSION} PIR_ELE_SIZE_EXP=${ELEMENT_SIZE_EXP} PIR_NUM_SHARDS=${NUM_SHARDS}
DB_GEN_PRELIM=DB_ALL_ONES=${DB_ALL_ONES} DB_NUM_ENTRIES_EXP=${DB_NUM_ENTRIES_EXP} DB_OUTPUT_PATH=${DB_FILE_PATH} DB_ELEMENT_SIZE_EXP=${ELEMENT_SIZE_EXP}

LIB_PRELIM=${DB_FILE_PRELIM}
BIN_PRELIM=${BIN_DB_FILE_PRELIM} ${PARAMS_OUTPUT_PATH_PRELIM}

CARGO=cargo
CARGO_COMMAND=${PRELIM} ${CARGO}
PYTHON_COMMAND=${DB_GEN_PRELIM} python3

.PHONY: gen-db
gen-db:
	${PYTHON_COMMAND} data/generate_db.py

.PHONY: build test bench bench-all
build:
	${CARGO_COMMAND} build
test:
	${CARGO_COMMAND} test
bench:
	${PRELIM} ${PIR_ENV} ${CARGO} bench
bench-all:
	${PRELIM} ${PIR_ENV_ALL} PIR_MATRIX_HEIGHT_EXP=16 PIR_PLAINTEXT_BITS=10 ${CARGO} bench > benchmarks-16.txt
	${PRELIM} ${PIR_ENV_ALL} PIR_MATRIX_HEIGHT_EXP=17 PIR_PLAINTEXT_BITS=10 ${CARGO} bench > benchmarks-17.txt
	${PRELIM} ${PIR_ENV_ALL} PIR_MATRIX_HEIGHT_EXP=18 PIR_PLAINTEXT_BITS=10 ${CARGO} bench > benchmarks-18.txt
	${PRELIM} ${PIR_ENV_ALL} PIR_MATRIX_HEIGHT_EXP=19 PIR_PLAINTEXT_BITS=9 ${CARGO} bench > benchmarks-19.txt
	${PRELIM} ${PIR_ENV_ALL} PIR_MATRIX_HEIGHT_EXP=20 PIR_PLAINTEXT_BITS=9 ${CARGO} bench > benchmarks-20.txt

# local environment make steps
.PHONY: prepare prepare-buckets prepare-server-commands run-server query build-docker
prepare:
	make build-docker
	make prepare-buckets

run-server:
	make prepare-buckets
	make prepare-server-commands
	./${START_CONTAINERS_SCRIPT_PATH}

query:
	docker run --network='host' -v ${ROOT_DIR}/data:/pir/data \
		-e CONFIG=${LOCAL_CONFIGS} -e USERNAME=${USERNAME} -e PWD=${PWD} \
		pir-client

build-docker:
	 docker build . -f infra/rust/server/Dockerfile -t pir-server
	 docker build . -f infra/rust/client/Dockerfile -t pir-client
	 docker build . -f infra/go/creds-wrangling-utils/Dockerfile -t prepare-buckets
	 docker build . -f infra/go/localmanager/Dockerfile -t localmanager

prepare-buckets:
	docker run -v ${ROOT_DIR}/data:/app/data -e LOCAL_CONFIGS=${LOCAL_CONFIGS} prepare-buckets

prepare-server-commands:
	docker run -v ${ROOT_DIR}/data:/app/data -e LOCAL_CONFIGS=${LOCAL_CONFIGS} -e PWD=${PWD}  \
		-e SCRIPT_PATH=${START_CONTAINERS_SCRIPT_PATH} localmanager

