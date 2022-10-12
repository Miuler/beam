package org.apache.beam.sdk.io.azure.cosmos

import com.azure.cosmos.CosmosClientBuilder
import org.apache.beam.sdk.testing.TestPipeline
import org.junit._
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.junit.runners.JUnit4
import org.slf4j.LoggerFactory
import org.testcontainers.containers.CosmosDBEmulatorContainer
import org.testcontainers.utility.DockerImageName

import java.nio.file.Files
import scala.annotation.meta.getter

@RunWith(classOf[JUnit4])
class CosmosIOTest {
  private val log = LoggerFactory.getLogger("CosmosIOTest")
  @(Rule @getter)
  val pipelineWrite: TestPipeline = TestPipeline.create
  @(Rule @getter)
  val pipelineRead: TestPipeline = TestPipeline.create

  @Test def test(): Unit = {
    ReadCosmosBuilder()
    val client = new CosmosClientBuilder().gatewayMode
      .endpointDiscoveryEnabled(false)
      .endpoint(CosmosIOTest.emulator.getEmulatorEndpoint)
      .key(CosmosIOTest.emulator.getEmulatorKey)
      .buildClient
    log.info("CosmosDB client created {}", client)
  }
}

@RunWith(classOf[JUnit4])
object CosmosIOTest {
  private val log = LoggerFactory.getLogger("CosmosIOTest[Obj]")
  private var emulator: CosmosDBEmulatorContainer = null

  @BeforeClass
  def setup(): Unit = {
    log.info("Starting CosmosDB emulator")
    emulator = new CosmosDBEmulatorContainer(
      DockerImageName.parse("mcr.microsoft.com/cosmosdb/linux/azure-cosmos-emulator:latest")
    )
    emulator.start()
    val tempFolder = new TemporaryFolder
    tempFolder.create()
    val keyStoreFile = tempFolder.newFile("azure-cosmos-emulator.keystore").toPath
    val keyStore = emulator.buildNewKeyStore
    keyStore.store(Files.newOutputStream(keyStoreFile.toFile.toPath), emulator.getEmulatorKey.toCharArray)
    System.setProperty("javax.net.ssl.trustStore", keyStoreFile.toString)
    System.setProperty("javax.net.ssl.trustStorePassword", emulator.getEmulatorKey)
    System.setProperty("javax.net.ssl.trustStoreType", "PKCS12")
  }

  @AfterClass def close(): Unit = {
    log.info("Stop CosmosDB emulator")
    emulator.close()
  }
}
