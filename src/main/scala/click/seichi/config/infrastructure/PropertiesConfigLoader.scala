package click.seichi.config.infrastructure

import cats.effect.Sync
import click.seichi.config.domain.ConfigLoader

import java.nio.charset.StandardCharsets
import java.nio.file.{Files, Paths}
import java.util.Properties

class PropertiesConfigLoader[F[_]: Sync] extends ConfigLoader[F, Properties] {

  private val filePath = "settings/config.properties"

  def load(): F[Properties] = Sync[F].delay {
    val properties = new Properties()
    properties.load(Files.newBufferedReader(Paths.get(filePath), StandardCharsets.UTF_8))
    properties
  }

}
