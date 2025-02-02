// Licensed to the Apache Software Foundation (ASF) under one or more
// contributor license agreements.  See the NOTICE file distributed with
// this work for additional information regarding copyright ownership.
// The ASF licenses this file to You under the Apache License, Version 2.0
// (the "License"); you may not use this file except in compliance with
// the License.  You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/apache/beam/sdks/v2/go/pkg/beam/artifact"
	fnpb "github.com/apache/beam/sdks/v2/go/pkg/beam/model/fnexecution_v1"
	pipepb "github.com/apache/beam/sdks/v2/go/pkg/beam/model/pipeline_v1"
	"google.golang.org/protobuf/proto"
)

func TestEnsureEndpointsSet_AllSet(t *testing.T) {
	provisionInfo := &fnpb.ProvisionInfo{
		LoggingEndpoint:  &pipepb.ApiServiceDescriptor{Url: "testLoggingEndpointUrl"},
		ArtifactEndpoint: &pipepb.ApiServiceDescriptor{Url: "testArtifactEndpointUrl"},
		ControlEndpoint:  &pipepb.ApiServiceDescriptor{Url: "testControlEndpointUrl"},
	}
	*loggingEndpoint = ""
	*artifactEndpoint = ""
	*controlEndpoint = ""
	err := ensureEndpointsSet(provisionInfo)
	if err != nil {
		t.Fatalf("ensureEndpointsSet() = %q, want nil", err)
	}
	if got, want := *loggingEndpoint, "testLoggingEndpointUrl"; got != want {
		t.Fatalf("After ensureEndpointsSet(), *loggingEndpoint = %q, want %q", got, want)
	}
	if got, want := *artifactEndpoint, "testArtifactEndpointUrl"; got != want {
		t.Fatalf("After ensureEndpointsSet(), *artifactEndpoint = %q, want %q", got, want)
	}
	if got, want := *controlEndpoint, "testControlEndpointUrl"; got != want {
		t.Fatalf("After ensureEndpointsSet(), *controlEndpoint = %q, want %q", got, want)
	}
}

func TestEnsureEndpointsSet_OneMissing(t *testing.T) {
	provisionInfo := &fnpb.ProvisionInfo{
		LoggingEndpoint:  &pipepb.ApiServiceDescriptor{Url: "testLoggingEndpointUrl"},
		ArtifactEndpoint: &pipepb.ApiServiceDescriptor{Url: "testArtifactEndpointUrl"},
		ControlEndpoint:  &pipepb.ApiServiceDescriptor{Url: ""},
	}
	*loggingEndpoint = ""
	*artifactEndpoint = ""
	*controlEndpoint = ""
	err := ensureEndpointsSet(provisionInfo)
	if err == nil {
		t.Fatalf("ensureEndpointsSet() = nil, want non-nil error")
	}
	if got, want := *loggingEndpoint, "testLoggingEndpointUrl"; got != want {
		t.Fatalf("After ensureEndpointsSet(), *loggingEndpoint = %q, want %q", got, want)
	}
	if got, want := *artifactEndpoint, "testArtifactEndpointUrl"; got != want {
		t.Fatalf("After ensureEndpointsSet(), *artifactEndpoint = %q, want %q", got, want)
	}
	if got, want := *controlEndpoint, ""; got != want {
		t.Fatalf("After ensureEndpointsSet(), *controlEndpoint = %q, want %q", got, want)
	}
}

func TestGetRustWorkerArtifactName_NoArtifacts(t *testing.T) {
	_, err := getRustWorkerArtifactName([]*pipepb.ArtifactInformation{})
	if err == nil {
		t.Fatalf("getRustWorkerArtifactName() = nil, want non-nil error")
	}
}

func TestGetRustWorkerArtifactName_OneArtifact(t *testing.T) {
	artifact := constructArtifactInformation(t, URNRustWorkerBinaryRole, "test/path", "sha")
	artifacts := []*pipepb.ArtifactInformation{&artifact}

	val, err := getRustWorkerArtifactName(artifacts)
	if err != nil {
		t.Fatalf("getRustWorkerArtifactName() = %v, want nil", err)
	}
	if got, want := val, "test/path"; got != want {
		t.Fatalf("getRustWorkerArtifactName() = %v, want %v", got, want)
	}
}

func TestGetRustWorkerArtifactName_MultipleArtifactsFirstIsWorker(t *testing.T) {
	artifact1 := constructArtifactInformation(t, URNRustWorkerBinaryRole, "test/path", "sha")
	artifact2 := constructArtifactInformation(t, "other role", "test/path2", "sha")
	artifacts := []*pipepb.ArtifactInformation{&artifact1, &artifact2}

	val, err := getRustWorkerArtifactName(artifacts)
	if err != nil {
		t.Fatalf("getRustWorkerArtifactName() = %v, want nil", err)
	}
	if got, want := val, "test/path"; got != want {
		t.Fatalf("getRustWorkerArtifactName() = %v, want %v", got, want)
	}
}

func TestGetRustWorkerArtifactName_MultipleArtifactsSecondIsWorker(t *testing.T) {
	artifact1 := constructArtifactInformation(t, "other role", "test/path", "sha")
	artifact2 := constructArtifactInformation(t, URNRustWorkerBinaryRole, "test/path2", "sha")
	artifacts := []*pipepb.ArtifactInformation{&artifact1, &artifact2}

	val, err := getRustWorkerArtifactName(artifacts)
	if err != nil {
		t.Fatalf("getRustWorkerArtifactName() = %v, want nil", err)
	}
	if got, want := val, "test/path2"; got != want {
		t.Fatalf("getRustWorkerArtifactName() = %v, want %v", got, want)
	}
}

func TestGetRustWorkerArtifactName_MultipleArtifactsNoneMatch(t *testing.T) {
	artifact1 := constructArtifactInformation(t, "other role", "test/path", "sha")
	artifact2 := constructArtifactInformation(t, "other role", "test/path2", "sha")
	artifacts := []*pipepb.ArtifactInformation{&artifact1, &artifact2}

	_, err := getRustWorkerArtifactName(artifacts)
	if err == nil {
		t.Fatalf("getRustWorkerArtifactName() = nil, want non-nil error")
	}
}

func TestCopyExe(t *testing.T) {
	testExeContent := []byte("testContent")

	// Make temp directory to cleanup at the end
	d, err := os.MkdirTemp(os.Getenv("TEST_TMPDIR"), "copyExe-*")
	if err != nil {
		t.Fatalf("failed to make temp directory, got %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(d) })

	// Make our source file and write to it
	src, err := os.CreateTemp(d, "src.exe")
	if err != nil {
		t.Fatalf("failed to make temp file, got %v", err)
	}
	if _, err := src.Write(testExeContent); err != nil {
		t.Fatalf("failed to write to temp file, got %v", err)
	}
	if err := src.Close(); err != nil {
		t.Fatalf("failed to close temp file, got %v", err)
	}

	// Make sure our destination path doesn't exist already
	srcPath, destPath := src.Name(), filepath.Join(d, "dest.exe")
	if _, err := os.Stat(destPath); err == nil {
		t.Fatalf("dest file %v already exists", destPath)
	}

	err = copyExe(srcPath, destPath)
	if err != nil {
		t.Fatalf("copyExe() = %v, want nil", err)
	}
	if _, err := os.Stat(destPath); err != nil {
		t.Fatalf("After running copyExe, os.Stat() = %v, want nil", err)
	}
	destContents, err := os.ReadFile(destPath)
	if err != nil {
		t.Fatalf("After running copyExe, os.ReadFile() = %v, want nil", err)
	}
	if got, want := string(destContents), string(testExeContent); got != want {
		t.Fatalf("After running copyExe, os.ReadFile() = %v, want %v", got, want)
	}
}

func constructArtifactInformation(t *testing.T, roleUrn string, path string, sha string) pipepb.ArtifactInformation {
	t.Helper()

	typePayload, _ := proto.Marshal(&pipepb.ArtifactFilePayload{Path: path, Sha256: sha})

	return pipepb.ArtifactInformation{
		RoleUrn:     roleUrn,
		TypeUrn:     artifact.URNFileArtifact,
		TypePayload: typePayload,
	}
}
