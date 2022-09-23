package click.seichi

import cats.effect.IO
import cats.effect.unsafe.implicits.global
import click.seichi.configuration.domain.DatabaseConnectionInformation

object SeichiTimedStatsConifers {

  private val config: configuration.System[IO] =
    configuration.System.getProperties[IO].unsafeRunSync()

  private lazy val databaseConnectionInformation: DatabaseConnectionInformation =
    config.api.getDatabaseConnectionInformation.unsafeRunSync()

  def main(args: Array[String]): Unit = {}

}
