package org.apache.beam.sdk.io.azure.cosmos

import com.azure.cosmos.models.CosmosQueryRequestOptions
import com.azure.cosmos.{CosmosClient, CosmosClientBuilder}
import org.apache.beam.sdk.io.BoundedSource
import org.bson.Document
import org.slf4j.LoggerFactory

import java.util


private class CosmosBoundedReader(cosmosSource: CosmosBoundedSource) extends BoundedSource.BoundedReader[Document] {
  private val log = LoggerFactory.getLogger(getClass)
  private var maybeClient: Option[CosmosClient] = None
//  private var maybeIterator: Option[java.util.Iterator[util.Map[String, Object]]] = None
  private var maybeIterator: Option[java.util.Iterator[Document]] = None

  override def start(): Boolean = {
    maybeClient = Some(
      new CosmosClientBuilder()
        .gatewayMode
        .endpointDiscoveryEnabled(false)
        .endpoint(cosmosSource.readCosmos.endpoint)
        .key(cosmosSource.readCosmos.key)
        .buildClient
    )

    maybeIterator = maybeClient.map { client =>
      log.info("Get the container name")
      val cosmosContainerProperties = client
        .getDatabase(cosmosSource.readCosmos.database)
        .queryContainers(cosmosSource.readCosmos.query)
        .iterator()
        .next()

      log.info(s"Get the iterator of the query in container ${cosmosContainerProperties.getId}")
      client
        .getDatabase(cosmosSource.readCosmos.database)
        .getContainer(cosmosContainerProperties.getId)
//        .queryItems(cosmosSource.readCosmos.query, new CosmosQueryRequestOptions(), classOf[util.Map[String, Object]])
        .queryItems(cosmosSource.readCosmos.query, new CosmosQueryRequestOptions(), classOf[Document])
        .iterator()
    }

    true
  }

  override def advance(): Boolean = maybeIterator.exists(_.hasNext)

  override def getCurrent: Document = maybeIterator
    .filter(_.hasNext)
//    .map(iterator => new Document(iterator.next()))
    .map(_.next())
    .orNull

  override def getCurrentSource: CosmosBoundedSource = cosmosSource

  override def close(): Unit = maybeClient.foreach(_.close())
}
