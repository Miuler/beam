package org.apache.beam.sdk.io.azure.cosmos

import org.apache.beam.sdk.io.Read
import org.apache.beam.sdk.transforms.PTransform
import org.apache.beam.sdk.values.{PBegin, PCollection}
import org.bson.Document
import org.slf4j.LoggerFactory

case class CosmosRead(private[cosmos] val endpoint: String = null,
                      private[cosmos] val key: String = null,
                      private[cosmos] val database: String = null,
                      private[cosmos] val query: String = null)
  extends PTransform[PBegin, PCollection[Document]] {


  private val log = LoggerFactory.getLogger(classOf[CosmosRead])

  /** Create new ReadCosmos based into previous ReadCosmos, modifying the endpoint */
  def withCosmosEndpoint(endpoint: String): CosmosRead = this.copy(endpoint = endpoint)

  def withCosmosKey(key: String): CosmosRead = this.copy(key = key)

  def withDatabase(database: String): CosmosRead = this.copy(database = database)

  def withQuery(query: String): CosmosRead = this.copy(query = query)

  override def expand(input: PBegin): PCollection[Document] = {
    log.debug(s"Read CosmosDB with endpoint: $endpoint and query: $query")
    // input.getPipeline.apply(Read.from(new CosmosSource(this)))
    input.apply(Read.from(new CosmosBoundedSource(this)))
  }
}
