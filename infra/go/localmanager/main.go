package main

import (
	"fmt"
	"os"

	yaml "gopkg.in/yaml.v3"
)

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

	commands := ""

	for i, instance := range configs.Instances {
		execString := fmt.Sprintf(
			"docker run --name %v --network='host' -v %v/data:/pir/data -p %v:%v -e ENV=local -e PORT=%v -e SHARDS_INTERVAL=%v, -e OPRF_KEY=%v, -e SHARD_DIR=%v pir-server &\n\n",
			fmt.Sprintf("pir-server-%v", i),
			pwd,
			instance.Port,
			instance.Port,
			instance.Port,
			instance.Shards,
			configs.OPRFKey,
			configs.BucketsPath,
		)

		commands = commands + execString

		printCommand(execString, configs, instance.Port, instance.Shards)
	}

	execShellPath := os.Getenv("SCRIPT_PATH")

	nf, err := os.OpenFile(execShellPath, os.O_RDWR|os.O_CREATE|os.O_TRUNC, 0755)
	if err != nil {
		panic(fmt.Sprintf("Error creating/re-writing start-server-containers.sh file: %v", err))
	}

	fmt.Println("\nWriting shell script to start docker containers to file...")

	shellScript := "#!/bin/bash \n\n"
	shellScript = shellScript + commands

	_, err = nf.WriteString(shellScript)
	if err != nil {
		panic(fmt.Sprintf("Error writing shell script file to disk: %v", err))
	}

	fmt.Println("Done")
}

func printCommand(execString string, configs Configs, port, shards string) {
	fmt.Println(fmt.Sprintf(
		">> Configured server instance for docker container with port=%v, shards=%v, oprf_key=%v.\n command: %v",
		port, shards, configs.OPRFKey, execString,
	))
}
