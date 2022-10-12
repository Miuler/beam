package org.apache.beam.sdk.io.azure.cosmos

import org.apache.beam.sdk.io.Read
import org.apache.beam.sdk.transforms.PTransform
import org.apache.beam.sdk.values.{ PBegin, PCollection }

case class ReadCosmos(endpoint: String = null) extends PTransform[PBegin, PCollection[String]] {

  /** Create new ReadCosmos based into previous ReadCosmos, modifying the endpoint */
  def withCosmosEndpoint(endpoint: String): ReadCosmos =
    this.copy(endpoint = endpoint)

  override def expand(input: PBegin): PCollection[String] = {
    // input.getPipeline.apply(Read.from(new CosmosSource(this)))
    input.apply(Read.from(new CosmosSource(this)))
  }
}
