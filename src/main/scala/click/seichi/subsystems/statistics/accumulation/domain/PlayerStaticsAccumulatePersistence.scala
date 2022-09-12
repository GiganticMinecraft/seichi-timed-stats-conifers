package click.seichi.subsystems.statistics.accumulation.domain

trait PlayerStaticsAccumulatePersistence[F[_]] {

  /**
   * @return 全プレイヤーの整地量を蓄積する作用
   */
  def accumulateSeichiAmountForAllPlayers(): F[Unit]

  /**
   * @return 全プレイヤーの建築量を蓄積する作用
   */
  def accumulateBuildAmountForAllPlayers(): F[Unit]

  /**
   * @return 全プレイヤーのプレイティック数を蓄積する作用
   */
  def accumulatePlayTicksForAllPlayers(): F[Unit]

  /**
   * @return 全プレイヤーの投票数を蓄積数作用
   */
  def accumulateVoteCountForAllPlayers(): F[Unit]

}
