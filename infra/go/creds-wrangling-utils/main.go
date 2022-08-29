package main

import (
	"bufio"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"fmt"
	"io/ioutil"
	"log"
	"os"
	"strconv"
	"strings"

	_ "github.com/aws/aws-sdk-go/service/s3"
	"github.com/cloudflare/circl/oprf"
	yaml "gopkg.in/yaml.v3"
)

const HASH_PREFIX_LEN = 16

type ProcessedCredential struct {
	RawUser    string
	RawPwd     string
	HashedUser string
	OprfEntry  string
	HashPrefix string
	Bucket     int64
}

type Confs struct {
	CredsPath     string
	BucketsPath   string
	NumberBuckets int
	KeyOPRF       oprf.PrivateKey
}

type fileConfs struct {
	ContentPath        string `yaml:"db_content_path"`
	BucketsPath        string `yaml:"buckets_path"`
	BucketsPerInstance int    `yaml:"buckets_per_instance"`
	OPRFKey            string `yaml:"oprf_key"`
	Instances          []struct {
		Port   string `yaml:"port"`
		Shards string `yaml:"shards"`
	}
}

func main() {
	confs := getConfigsEnv()

	// inits all buckets files, to make sure they exist even if empty
	initBucketFiles(confs.NumberBuckets, confs.BucketsPath)

	files, err := ioutil.ReadDir(confs.CredsPath)
	if err != nil {
		log.Fatal("Error reading from credential directory: ", confs.CredsPath)
	}

	keyOPRF := confs.KeyOPRF
	evaluatorOPRF := oprf.NewServer(oprf.SuiteP256, &keyOPRF)
	nStored := 0
	for i, f := range files {
		filePath := fmt.Sprintf("%v/%v", confs.CredsPath, f.Name())

		fmt.Printf("%v/%v | ", i, len(files))

		fd, err := os.Open(filePath)
		if err != nil {
			log.Fatal("Error opening file", err)
		}

		n := processCredentialsFile(fd, confs.NumberBuckets, confs.BucketsPath, evaluatorOPRF)
		nStored += n
	}

	log.Printf("Finished storing %v credentials\n", nStored)
}

func processCredentialsFile(fd *os.File, nBuckets int, bucketsPath string, evaluatorOPRF oprf.Server) int {
	scanner := bufio.NewScanner(fd)
	nStored := 0
	for scanner.Scan() {
		// split username and password
		creds := strings.Split(
			scanner.Text(),
			":",
		)

		processedCred := ProcessedCredential{RawUser: creds[0], RawPwd: creds[1]}
		processedCred.SetBucket(nBuckets, evaluatorOPRF)

		// store credential
		if err := processedCred.Store(bucketsPath); err != nil {
			log.Fatalf("Error storing processed credential %v: %v", creds, err)
		}
		nStored += 1
	}

	if err := scanner.Err(); err != nil {
		log.Fatal("Error streaming through file: ", err)
	}

	return nStored
}

func (c *ProcessedCredential) SetBucket(nBuckets int, evaluatorOPRF oprf.Server) {
	h := sha256.New()
	h.Write([]byte(c.RawUser))
	c.HashedUser = fmt.Sprintf("%x", h.Sum(nil))

	cred := append([]byte(c.RawUser), []byte(c.RawPwd)...)
	// Evaluate PRF to create DB row
	oprfOutput, err := evaluatorOPRF.FullEvaluate(cred)
	if err != nil {
		log.Fatalf("Error evaluating OPRF key while processing credential %v, error: %v", cred, err)
	}
	c.OprfEntry = base64.StdEncoding.EncodeToString(oprfOutput)
	// Evaluate Hash to create LocalHashPrefix mapping table
	h2 := sha256.New()
	h2.Write(cred)
	fullHash := h2.Sum(nil)
	c.HashPrefix = base64.StdEncoding.EncodeToString(fullHash[:HASH_PREFIX_LEN])

	c.Bucket = calculatesBucketNumber(c.HashedUser, nBuckets)
}

func (c *ProcessedCredential) Store(dataPath string) error {
	// create buckets folder if it does not exist
	if _, err := os.Stat(dataPath); errors.Is(err, os.ErrNotExist) {
		if err := os.Mkdir(dataPath, 0755); err != nil {
			return fmt.Errorf("error creating buckets directory: %v", err)
		}
	}

	bucketPath := fmt.Sprintf("%v/%v.bucket", dataPath, c.Bucket)
	lhpPath := fmt.Sprintf("%v/%v.lhp", dataPath, c.Bucket)

	// open bucket and lhp files
	fdBucket, err := os.OpenFile(bucketPath, os.O_RDWR|os.O_APPEND|os.O_CREATE, 0660)
	if err != nil {
		return fmt.Errorf("error opening credential %v: %v", c.RawUser, err)
	}
	defer fdBucket.Close()
	fdLHP, err := os.OpenFile(lhpPath, os.O_RDWR|os.O_APPEND|os.O_CREATE, 0660)
	if err != nil {
		return fmt.Errorf("error opening credential %v: %v", c.RawUser, err)
	}
	defer fdLHP.Close()

	_, err = fdBucket.WriteString(fmt.Sprintf("%v\n", c.OprfEntry))
	if err != nil {
		return err
	}
	_, err = fdLHP.WriteString(fmt.Sprintf("%v\n", c.HashPrefix))
	return err
}

func (c ProcessedCredential) String() string {
	return c.RawUser
}

func getConfigsEnv() Confs {
	configPath := os.Getenv("LOCAL_CONFIGS")
	if configPath == "" {
		return configsFromEnv()
	}

	return configsFromFile(configPath)
}

func configsFromFile(configPath string) Confs {
	f, err := os.ReadFile(configPath)
	if err != nil {
		panic(fmt.Sprintf("Error opening local config file (%v) set in LOCAL_CONFIGS: %v",
			configPath, err))
	}

	configs := fileConfs{}

	err = yaml.Unmarshal(f, &configs)
	if err != nil {
		panic(fmt.Sprintf("Error loading configs: %v", err))
	}

	shards := []string{}
	for _, instances := range configs.Instances {
		shards = append(shards, instances.Shards)
	}

	numberBuckets := configs.BucketsPerInstance * len(configs.Instances)

	return Confs{
		CredsPath:     configs.ContentPath,
		BucketsPath:   configs.BucketsPath,
		NumberBuckets: numberBuckets,
		KeyOPRF:       oprfKeyFromString(configs.OPRFKey),
	}
}

func configsFromEnv() Confs {
	credsPath := os.Getenv("CREDS_PATH")
	if credsPath == "" {
		log.Fatal("Required env variable CREDS_PATH is not defined")
	}

	bucketsPath := os.Getenv("BUCKETS_PATH")
	if bucketsPath == "" {
		log.Fatal("Required env variable BUCKETS_PATH is not defined")
	}

	nBucketsStr := os.Getenv("NUMBER_BUCKETS")
	if nBucketsStr == "" {
		log.Fatal("Required env variable NUMBER_BUCKETS is not defined")
	}
	nBuckets, err := strconv.Atoi(nBucketsStr)
	if err != nil {
		log.Fatal("NUMBER_BUCKETS has a wrong format", err)
	}

	keyOPRFBase64 := os.Getenv("OPRF_KEY")
	if keyOPRFBase64 == "" {
		log.Fatal("Required env variable OPRF_KEY is not defined")
	}

	return Confs{
		CredsPath:     credsPath,
		BucketsPath:   bucketsPath,
		NumberBuckets: nBuckets,
		KeyOPRF:       oprfKeyFromString(keyOPRFBase64),
	}
}

func oprfKeyFromString(keyOPRFBase64 string) oprf.PrivateKey {
	decoded, err := base64.StdEncoding.DecodeString(keyOPRFBase64)
	if err != nil {
		log.Fatal("Failed to base64 decode OPRF_KEY: ", err)
	}
	keyOPRF := new(oprf.PrivateKey)
	err = keyOPRF.UnmarshalBinary(oprf.SuiteP256, decoded)
	if err != nil {
		log.Fatal("Failed to derive OPRF key: ", err)
	}

	return *keyOPRF
}

func calculatesBucketNumber(key string, nBuckets int) int64 {
	hexSum, err := strconv.ParseInt(key[:15], 16, 64)
	if err != nil {
		log.Fatalf("Unexpected error calculating bucket of %v: %v", key, err)
	}

	return hexSum % int64(nBuckets)
}

func initBucketFiles(nBuckets int, bucketsPath string) {
	for i := 0; i < nBuckets; i++ {
		bucketFilePath := fmt.Sprintf("%v/%v", bucketsPath, fmt.Sprintf("%v.bucket", i))
		lhpFilePath := fmt.Sprintf("%v/%v", bucketsPath, fmt.Sprintf("%v.lhp", i))

		_, err := os.Create(bucketFilePath)
		if err != nil {
			panic(fmt.Sprintf("Error creating bucket file: %v", err))
		}
		_, err = os.Create(lhpFilePath)
		if err != nil {
			panic(fmt.Sprintf("Error creating lhp file: %v", err))
		}
	}
}
