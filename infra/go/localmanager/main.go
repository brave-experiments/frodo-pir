package main

import (
	"fmt"
	"os"
	"os/exec"
	"sync"

	yaml "gopkg.in/yaml.v3"
)

var wg sync.WaitGroup

type Configs struct {
	ContentPath string `yaml:"db_content_path"`
	BucketsPath string `yaml:"buckets_path"`
	OPRFKey     string `yaml:"oprf_key"`
	Instances   []struct {
		Port   string `yaml:"port"`
		Shards string `yaml:"shards"`
	}
}

func main() {
	pwd := os.Getenv("PWD")
	if pwd == "" {
		panic("PWD should be defined")
	}

	configPath := os.Getenv("LOCAL_CONFIGS")
	f, err := os.ReadFile(configPath)
	if err != nil {
		panic(fmt.Sprintf("Error opening local config file (%v) set in LOCAL_CONFIGS: %v",
			configPath, err))
	}

	configs := Configs{}

	err = yaml.Unmarshal(f, &configs)
	if err != nil {
		panic(fmt.Sprintf("Error loading configs: %v", err))
	}

	fmt.Println("\n>> FrodoPIR local test environment:")
	fmt.Println("- DB path:", configs.ContentPath)
	fmt.Println("- Buckets path:", configs.BucketsPath)
	fmt.Println("- OPRF key:", configs.OPRFKey)
	fmt.Println("")

	wg.Add(len(configs.Instances))

	for i, instance := range configs.Instances {

		execString := fmt.Sprintf(
			"docker run --name %v --network='host' -v %v/test_data:/pir/test_data -p %v:%v -e ENV=local -e PORT=%v -e SHARDS_INTERVAL=%v, -e OPRF_KEY=%v, -e SHARD_DIR=%v server-instance",
			fmt.Sprintf("server-instance-%v", i),
			pwd,
			instance.Port,
			instance.Port,
			instance.Port,
			instance.Shards,
			configs.OPRFKey,
			configs.BucketsPath,
		)

		go spinupContainer(execString, configs, instance.Port, instance.Shards)
	}

	wg.Wait()
	fmt.Println("\nDone")
}

func spinupContainer(execString string, configs Configs, port, shards string) {
	defer wg.Done()

	fmt.Println(fmt.Sprintf(
		">> Starting server instance on a docker container with port=%v, shards=%v, oprf_key=%v. (command: %v)\n",
		port, shards, configs.OPRFKey, execString,
	))
	exec.Command("bash", "-c", execString).Output()
}
