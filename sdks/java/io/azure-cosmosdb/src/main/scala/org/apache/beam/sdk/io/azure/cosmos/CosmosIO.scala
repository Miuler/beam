package org.apache.beam.sdk.io.azure.cosmos;

import org.apache.beam.sdk.io.Read
import org.apache.beam.sdk.transforms.PTransform
import org.apache.beam.sdk.values.{ PBegin, PCollection };

object CosmosIO {
  def read(): ReadCosmos = {
    ReadCosmos();
  }
}

