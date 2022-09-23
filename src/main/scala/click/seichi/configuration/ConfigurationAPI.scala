package click.seichi.configuration

import click.seichi.configuration.domain.DatabaseConnectionInformation

trait ConfigurationAPI[F[_]] {

  /**
   * @return [[DatabaseConnectionInformation]]を返す作用
   */
  def getDatabaseConnectionInformation: F[DatabaseConnectionInformation]

}
