import {
  Controller,
  Get,
  Post,
  Delete,
  Param,
  Query,
  Body,
  UseGuards,
  HttpCode,
  HttpStatus,
  DefaultValuePipe,
  ParseIntPipe,
} from '@nestjs/common';
import { LaunchpadService } from './launchpad.service';
import { JwtAuthGuard } from '../../common/guards/jwt-auth.guard';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { User } from '../../database/entities/user.entity';
import {
  CreatePresaleContributionDto,
  SentimentVoteDto,
  AuditCommentDto,
  LaunchpadSubmissionDto,
} from './dto';

@Controller('launchpad')
export class LaunchpadController {
  constructor(private readonly launchpadService: LaunchpadService) {}

  @Get('gems')
  @HttpCode(HttpStatus.OK)
  async getGems(
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit: number,
    @Query('category') category?: string,
    @Query('verified') verified?: string,
    @Query('rugChecked') rugChecked?: string,
    @Query('minScore') minScore?: number,
    @Query('maxScore') maxScore?: number,
    @Query('minLiquidity') minLiquidity?: number,
    @Query('maxLiquidity') maxLiquidity?: number,
    @Query('minPriceChange') minPriceChange?: number,
    @Query('maxPriceChange') maxPriceChange?: number,
    @Query('search') search?: string,
  ) {
    return this.launchpadService.getGems({
      page,
      limit,
      category,
      verified,
      rugChecked,
      minScore: minScore !== undefined ? Number(minScore) : undefined,
      maxScore: maxScore !== undefined ? Number(maxScore) : undefined,
      minLiquidity: minLiquidity !== undefined ? Number(minLiquidity) : undefined,
      maxLiquidity: maxLiquidity !== undefined ? Number(maxLiquidity) : undefined,
      minPriceChange: minPriceChange !== undefined ? Number(minPriceChange) : undefined,
      maxPriceChange: maxPriceChange !== undefined ? Number(maxPriceChange) : undefined,
      search,
    });
  }

  @Get('gems/featured')
  @HttpCode(HttpStatus.OK)
  async getFeaturedGem() {
    return this.launchpadService.getFeaturedGem();
  }

  @Get('gems/stats')
  @HttpCode(HttpStatus.OK)
  async getStats() {
    return this.launchpadService.getGlobalStats();
  }

  @Get('gems/:address')
  @HttpCode(HttpStatus.OK)
  async getGemByAddress(@Param('address') address: string) {
    return this.launchpadService.getGemByAddress(address);
  }

  @Get('tokens/:address')
  @HttpCode(HttpStatus.OK)
  async getToken(@Param('address') address: string) {
    return this.launchpadService.getTokenDetail(address);
  }

  @Get('tokens/:address/price-history')
  @HttpCode(HttpStatus.OK)
  async getPriceHistory(
    @Param('address') address: string,
    @Query('range') range: string,
    @Query('points') points?: number,
  ) {
    return this.launchpadService.getPriceHistory(address, range || '24h', points);
  }

  @Get('tokens/:address/orderbook')
  @HttpCode(HttpStatus.OK)
  async getOrderbook(@Param('address') address: string) {
    return this.launchpadService.getOrderbook(address);
  }

  @Get('tokens/:address/tokenomics')
  @HttpCode(HttpStatus.OK)
  async getTokenomics(@Param('address') address: string) {
    return this.launchpadService.getTokenomics(address);
  }

  @Get('tokens/:address/sentiment')
  @HttpCode(HttpStatus.OK)
  async getSentiment(@Param('address') address: string) {
    return this.launchpadService.getSentiment(address);
  }

  @Post('tokens/:address/sentiment/vote')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async voteSentiment(
    @Param('address') address: string,
    @CurrentUser() user: User,
    @Body() dto: SentimentVoteDto,
  ) {
    await this.launchpadService.voteSentiment(address, user.id, dto.vote);
    return { data: true };
  }

  @Get('presales')
  @HttpCode(HttpStatus.OK)
  async getPresales(
    @Query('status') status?: string,
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page?: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit?: number,
    @Query('search') search?: string,
  ) {
    return this.launchpadService.getPresales({ status, page, limit, search });
  }

  @Get('presales/:id')
  @HttpCode(HttpStatus.OK)
  async getPresale(@Param('id') id: string) {
    return this.launchpadService.getPresaleById(id);
  }

  @Get('presales/:id/contributions')
  @HttpCode(HttpStatus.OK)
  async getPresaleContributions(
    @Param('id') id: string,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit: number,
  ) {
    return this.launchpadService.getPresaleContributions(id, limit);
  }

  @Post('presales/:id/contributions')
  @HttpCode(HttpStatus.CREATED)
  async createPresaleContribution(
    @Param('id') id: string,
    @Body() dto: CreatePresaleContributionDto,
  ) {
    return this.launchpadService.createPresaleContribution(
      id,
      dto.walletAddress,
      dto.amount,
      dto.txHash,
    );
  }

  @Get('contributions/me')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getMyContributions(
    @CurrentUser() user: User,
    @Query('status') status?: string,
    @Query('search') search?: string,
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page?: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit?: number,
  ) {
    return this.launchpadService.getContributionsByWallet(user.walletAddress, {
      status,
      search,
      page,
      limit,
    });
  }

  @Get('contributions/by-tx/:hash')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getContributionByTx(@Param('hash') hash: string, @CurrentUser() user: User) {
    return this.launchpadService.getContributionByTx(hash, user.walletAddress);
  }

  @Get('contributions/:id')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getContributionById(@Param('id') id: string, @CurrentUser() user: User) {
    return this.launchpadService.getContributionById(id, user.walletAddress);
  }

  @Get('audit/:address')
  @HttpCode(HttpStatus.OK)
  async getAudit(@Param('address') address: string) {
    return this.launchpadService.getAudit(address);
  }

  @Post('audit/:address/comment')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.CREATED)
  async addAuditComment(
    @Param('address') address: string,
    @CurrentUser() user: User,
    @Body() dto: AuditCommentDto,
  ) {
    return this.launchpadService.addAuditComment(address, user.walletAddress, dto.text);
  }

  @Get('watchlist')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getWatchlist(@CurrentUser() user: User) {
    return this.launchpadService.getWatchlist(user.id);
  }

  @Post('watchlist')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async addWatchlist(@CurrentUser() user: User, @Body('contractAddress') contractAddress: string) {
    await this.launchpadService.addWatchlist(user.id, contractAddress);
    return { data: true };
  }

  @Delete('watchlist/:address')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async removeWatchlist(@CurrentUser() user: User, @Param('address') address: string) {
    await this.launchpadService.removeWatchlist(user.id, address);
    return { data: true };
  }

  @Post('submissions')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.CREATED)
  async createSubmission(
    @CurrentUser() user: User,
    @Body() dto: LaunchpadSubmissionDto,
  ) {
    return this.launchpadService.createSubmission(user.id, dto);
  }

  @Get('submissions/:id')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getSubmission(@CurrentUser() user: User, @Param('id') id: string) {
    return this.launchpadService.getSubmission(user.id, id);
  }
}
