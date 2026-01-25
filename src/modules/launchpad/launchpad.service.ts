import { randomUUID } from 'node:crypto';
import {
  Injectable,
  NotFoundException,
  BadRequestException,
  ForbiddenException,
  OnModuleInit,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { LaunchpadGem } from '../../database/entities/launchpad-gem.entity';
import { LaunchpadFeaturedGem } from '../../database/entities/launchpad-featured-gem.entity';
import { LaunchpadPresale } from '../../database/entities/launchpad-presale.entity';
import { LaunchpadContribution } from '../../database/entities/launchpad-contribution.entity';
import { LaunchpadToken } from '../../database/entities/launchpad-token.entity';
import { LaunchpadAudit } from '../../database/entities/launchpad-audit.entity';
import { LaunchpadAuditComment } from '../../database/entities/launchpad-audit-comment.entity';
import { LaunchpadWatchlist } from '../../database/entities/launchpad-watchlist.entity';
import { LaunchpadSubmission } from '../../database/entities/launchpad-submission.entity';
import { LaunchpadTokenVote } from '../../database/entities/launchpad-token-vote.entity';
import {
  ContributionStatus,
  SentimentVote,
  SubmissionStatus,
} from '../../common/enums/launchpad.enum';
import { LaunchpadGateway } from './launchpad.gateway';

@Injectable()
export class LaunchpadService implements OnModuleInit {
  constructor(
    @InjectRepository(LaunchpadGem)
    private readonly gemRepository: Repository<LaunchpadGem>,
    @InjectRepository(LaunchpadFeaturedGem)
    private readonly featuredRepository: Repository<LaunchpadFeaturedGem>,
    @InjectRepository(LaunchpadPresale)
    private readonly presaleRepository: Repository<LaunchpadPresale>,
    @InjectRepository(LaunchpadContribution)
    private readonly contributionRepository: Repository<LaunchpadContribution>,
    @InjectRepository(LaunchpadToken)
    private readonly tokenRepository: Repository<LaunchpadToken>,
    @InjectRepository(LaunchpadAudit)
    private readonly auditRepository: Repository<LaunchpadAudit>,
    @InjectRepository(LaunchpadAuditComment)
    private readonly auditCommentRepository: Repository<LaunchpadAuditComment>,
    @InjectRepository(LaunchpadWatchlist)
    private readonly watchlistRepository: Repository<LaunchpadWatchlist>,
    @InjectRepository(LaunchpadSubmission)
    private readonly submissionRepository: Repository<LaunchpadSubmission>,
    @InjectRepository(LaunchpadTokenVote)
    private readonly voteRepository: Repository<LaunchpadTokenVote>,
    private readonly launchpadGateway: LaunchpadGateway,
  ) {}

  async onModuleInit() {
    await this.seedDefaults();
  }

  async getGems(query: {
    page?: number;
    limit?: number;
    category?: string;
    verified?: string;
    rugChecked?: string;
    minScore?: number;
    maxScore?: number;
    minLiquidity?: number;
    maxLiquidity?: number;
    minPriceChange?: number;
    maxPriceChange?: number;
    search?: string;
  }) {
    const page = query.page || 1;
    const limit = query.limit || 20;
    const qb = this.gemRepository.createQueryBuilder('gem');

    if (query.category) {
      qb.andWhere('gem.category = :category', { category: query.category });
    }
    if (query.verified !== undefined) {
      qb.andWhere('gem.isVerified = :verified', {
        verified: query.verified === 'true',
      });
    }
    if (query.rugChecked !== undefined) {
      qb.andWhere('gem.rugChecked = :rugChecked', {
        rugChecked: query.rugChecked === 'true',
      });
    }
    if (query.minScore !== undefined) {
      qb.andWhere('gem.securityScore >= :minScore', { minScore: query.minScore });
    }
    if (query.maxScore !== undefined) {
      qb.andWhere('gem.securityScore <= :maxScore', { maxScore: query.maxScore });
    }
    if (query.minLiquidity !== undefined) {
      qb.andWhere('gem.liquidityValue >= :minLiquidity', {
        minLiquidity: query.minLiquidity,
      });
    }
    if (query.maxLiquidity !== undefined) {
      qb.andWhere('gem.liquidityValue <= :maxLiquidity', {
        maxLiquidity: query.maxLiquidity,
      });
    }
    if (query.minPriceChange !== undefined) {
      qb.andWhere('gem.priceChange >= :minPriceChange', {
        minPriceChange: query.minPriceChange,
      });
    }
    if (query.maxPriceChange !== undefined) {
      qb.andWhere('gem.priceChange <= :maxPriceChange', {
        maxPriceChange: query.maxPriceChange,
      });
    }
    if (query.search) {
      qb.andWhere(
        '(LOWER(gem.projectName) LIKE LOWER(:search) OR LOWER(gem.contractAddress) LIKE LOWER(:search))',
        { search: `%${query.search}%` },
      );
    }

    qb.orderBy('gem.createdAt', 'DESC')
      .skip((page - 1) * limit)
      .take(limit);

    const [gems, total] = await qb.getManyAndCount();

    return {
      data: gems.map((gem) => this.mapGem(gem)),
      total,
      page,
      limit,
      totalPages: Math.ceil(total / limit),
    };
  }

  async getFeaturedGem() {
    const featured = await this.featuredRepository.findOne({
      order: { updatedAt: 'DESC' },
    });

    if (!featured) {
      return { data: this.defaultFeaturedGem() };
    }

    return { data: this.mapFeaturedGem(featured) };
  }

  async getGlobalStats() {
    const totalGems = await this.gemRepository.count();
    const liquiditySum = await this.gemRepository
      .createQueryBuilder('gem')
      .select('COALESCE(SUM(gem.liquidityValue), 0)', 'sum')
      .getRawOne();
    const avgScore = await this.gemRepository
      .createQueryBuilder('gem')
      .select('COALESCE(AVG(gem.securityScore), 0)', 'avg')
      .getRawOne();
    const activePresales = await this.presaleRepository
      .createQueryBuilder('presale')
      .where('presale.endDate > :now', { now: new Date() })
      .getCount();
    const volumeSum = await this.gemRepository
      .createQueryBuilder('gem')
      .select('COALESCE(SUM(gem.volume24h), 0)', 'sum')
      .getRawOne();

    return {
      data: {
        totalGems,
        totalLiquidity: this.formatCurrency(Number(liquiditySum.sum)),
        avgSecurityScore: Number(avgScore.avg),
        activePresales,
        totalVolume24h: this.formatCurrency(Number(volumeSum.sum)),
      },
    };
  }

  async getGemByAddress(address: string) {
    const gem = await this.gemRepository
      .createQueryBuilder('gem')
      .where('LOWER(gem.contractAddress) = LOWER(:address)', { address })
      .getOne();

    if (!gem) {
      throw new NotFoundException('Gem no encontrado');
    }

    return { data: this.mapGem(gem) };
  }

  async getTokenDetail(address: string) {
    const token = await this.tokenRepository
      .createQueryBuilder('token')
      .where('LOWER(token.contractAddress) = LOWER(:address)', { address })
      .getOne();

    const gem = await this.gemRepository
      .createQueryBuilder('gem')
      .where('LOWER(gem.contractAddress) = LOWER(:address)', { address })
      .getOne();

    if (!token && !gem) {
      throw new NotFoundException('Token no encontrado');
    }

    const base = token || ({} as LaunchpadToken);
    const tokenomics = base.tokenomics || {
      totalSupply: '0',
      burned: '0',
      devWalletLockDays: 0,
    };
    const daoSentiment = await this.getSentiment(address);

    return {
      data: {
        projectIcon: token?.projectIcon || gem?.projectIcon || '',
        projectName: token?.projectName || gem?.projectName || '',
        symbol: token?.symbol || gem?.projectName || '',
        price: Number(token?.price || gem?.price || 0),
        priceChange24h: Number(token?.priceChange24h || gem?.priceChange || 0),
        isVerified: token?.isVerified ?? gem?.isVerified ?? false,
        contractAddress: address,
        exchangeRate: Number(token?.exchangeRate || 0),
        sparklineData: token?.sparklineData || gem?.sparklineData || [],
        tokenomics,
        daoSentiment: daoSentiment.data,
      },
    };
  }

  async getPriceHistory(address: string, range: string, points?: number) {
    const token = await this.tokenRepository
      .createQueryBuilder('token')
      .where('LOWER(token.contractAddress) = LOWER(:address)', { address })
      .getOne();
    const basePrice = Number(token?.price || 0);
    const totalPoints = points || (range === '24h' ? 24 : range === '7d' ? 14 : 30);
    const now = Date.now();
    const stepMs = range === '24h' ? 60 * 60 * 1000 : 24 * 60 * 60 * 1000;
    const series = Array.from({ length: totalPoints }).map((_, idx) => {
      const time = new Date(now - stepMs * (totalPoints - idx)).toISOString();
      const wave = Math.sin(idx / 2) * 0.02;
      const price = Math.max(0, basePrice + basePrice * wave);
      return { time, price, volume: 0 };
    });

    return { data: series };
  }

  async getOrderbook(address: string) {
    const token = await this.tokenRepository
      .createQueryBuilder('token')
      .where('LOWER(token.contractAddress) = LOWER(:address)', { address })
      .getOne();
    const basePrice = Number(token?.price || 0.0001);
    const sellOffers = Array.from({ length: 5 }).map((_, idx) => ({
      price: (basePrice * (1 + idx * 0.01)).toFixed(8),
      amount: (1000 - idx * 50).toFixed(2),
      score: 80 - idx * 2,
      address: `0xSELL${idx}`,
    }));
    const buyOffers = Array.from({ length: 5 }).map((_, idx) => ({
      price: (basePrice * (1 - idx * 0.01)).toFixed(8),
      amount: (900 - idx * 45).toFixed(2),
      score: 78 - idx * 2,
      address: `0xBUY${idx}`,
    }));

    return { data: { sellOffers, buyOffers } };
  }

  async getTokenomics(address: string) {
    const token = await this.tokenRepository
      .createQueryBuilder('token')
      .where('LOWER(token.contractAddress) = LOWER(:address)', { address })
      .getOne();
    const tokenomics = token?.tokenomics || {
      totalSupply: '0',
      burned: '0',
      devWalletLockDays: 0,
    };
    return { data: tokenomics };
  }

  async getSentiment(address: string) {
    const votes = await this.voteRepository.count({
      where: { contractAddress: address, vote: SentimentVote.BULLISH },
    });
    const totalVotes = await this.voteRepository.count({
      where: { contractAddress: address },
    });
    const bearish = totalVotes - votes;
    const score = totalVotes === 0 ? 0 : Math.round((votes / totalVotes) * 100);
    const label = score >= 60 ? 'Bullish' : score <= 40 ? 'Bearish' : 'Neutral';

    return {
      data: {
        score,
        label,
        comments: [],
      },
    };
  }

  async voteSentiment(address: string, userId: string, vote: 'bullish' | 'bearish') {
    const existing = await this.voteRepository.findOne({
      where: { contractAddress: address, userId },
    });

    if (existing) {
      existing.vote = vote === 'bullish' ? SentimentVote.BULLISH : SentimentVote.BEARISH;
      await this.voteRepository.save(existing);
      return;
    }

    const newVote = this.voteRepository.create({
      contractAddress: address,
      userId,
      vote: vote === 'bullish' ? SentimentVote.BULLISH : SentimentVote.BEARISH,
    });
    await this.voteRepository.save(newVote);
  }

  async getPresales(query: { status?: string; page?: number; limit?: number; search?: string }) {
    const page = query.page || 1;
    const limit = query.limit || 20;
    const qb = this.presaleRepository.createQueryBuilder('presale');

    if (query.status) {
      if (query.status === 'active') {
        qb.andWhere('presale.endDate > :now', { now: new Date() });
      } else if (query.status === 'ended') {
        qb.andWhere('presale.endDate <= :now', { now: new Date() });
      }
    }
    if (query.search) {
      qb.andWhere(
        '(LOWER(presale.projectName) LIKE LOWER(:search) OR LOWER(presale.tokenSymbol) LIKE LOWER(:search))',
        { search: `%${query.search}%` },
      );
    }

    qb.orderBy('presale.endDate', 'ASC')
      .skip((page - 1) * limit)
      .take(limit);

    const [presales, total] = await qb.getManyAndCount();

    return {
      data: presales.map((presale) => this.mapPresale(presale)),
      total,
      page,
      limit,
      totalPages: Math.ceil(total / limit),
    };
  }

  async getPresaleById(id: string) {
    const presale = await this.presaleRepository.findOne({ where: { id } });
    if (!presale) {
      throw new NotFoundException('Presale no encontrada');
    }
    return { data: this.mapPresale(presale) };
  }

  async getPresaleContributions(id: string, limit: number = 20) {
    const contributions = await this.contributionRepository.find({
      where: { presaleId: id },
      order: { createdAt: 'DESC' },
      take: limit,
    });
    return {
      data: contributions.map((c) => ({
        id: c.id,
        walletAddress: c.walletAddress,
        amount: c.contributionAmount.toString(),
        timestamp: c.createdAt.toISOString(),
      })),
    };
  }

  async createPresaleContribution(
    presaleId: string,
    walletAddress: string,
    amount: string,
    txHash?: string,
  ) {
    const presale = await this.presaleRepository.findOne({ where: { id: presaleId } });
    if (!presale) {
      throw new NotFoundException('Presale no encontrada');
    }

    const amountNum = Number(amount);
    if (Number.isNaN(amountNum) || amountNum <= 0) {
      throw new BadRequestException('Amount inválido');
    }

    const contribution = this.contributionRepository.create({
      presaleId,
      walletAddress,
      projectName: presale.projectName,
      projectIcon: presale.projectIcon,
      tokenSymbol: presale.tokenSymbol,
      contributionAmount: amountNum,
      buyPrice: Number(presale.exchangeRate),
      currentValue: amountNum,
      growth: '1.0x Growth',
      isLoss: false,
      vestingProgress: 0,
      nextUnlock: 'TBA',
      claimableAmount: null,
      status: ContributionStatus.ACTIVE,
      txHash,
    });

    const saved = await this.contributionRepository.save(contribution);

    this.launchpadGateway.emitPresaleContribution(presaleId, {
      id: saved.id,
      walletAddress: saved.walletAddress,
      amount: saved.contributionAmount.toString(),
      timestamp: saved.createdAt.toISOString(),
    });

    return { data: this.mapContribution(saved) };
  }

  async getContributionsByWallet(
    walletAddress: string,
    query: { status?: string; search?: string; page?: number; limit?: number },
  ) {
    const page = query.page || 1;
    const limit = query.limit || 20;
    const qb = this.contributionRepository.createQueryBuilder('contribution');

    qb.where('LOWER(contribution.walletAddress) = LOWER(:walletAddress)', {
      walletAddress,
    });
    if (query.status) {
      qb.andWhere('contribution.status = :status', { status: query.status });
    }
    if (query.search) {
      qb.andWhere(
        '(LOWER(contribution.projectName) LIKE LOWER(:search) OR LOWER(contribution.tokenSymbol) LIKE LOWER(:search))',
        { search: `%${query.search}%` },
      );
    }

    qb.orderBy('contribution.createdAt', 'DESC')
      .skip((page - 1) * limit)
      .take(limit);

    const [items, total] = await qb.getManyAndCount();

    return {
      data: items.map((c) => this.mapContribution(c)),
      total,
      page,
      limit,
      totalPages: Math.ceil(total / limit),
    };
  }

  async getContributionById(id: string, walletAddress: string) {
    const contribution = await this.contributionRepository.findOne({ where: { id } });
    if (!contribution) {
      throw new NotFoundException('Contribución no encontrada');
    }
    if (contribution.walletAddress.toLowerCase() !== walletAddress.toLowerCase()) {
      throw new ForbiddenException('No tienes permiso para ver esta contribución');
    }
    return { data: this.mapContribution(contribution) };
  }

  async getContributionByTx(hash: string, walletAddress: string) {
    const contribution = await this.contributionRepository.findOne({
      where: { txHash: hash },
    });
    if (!contribution) {
      throw new NotFoundException('Contribución no encontrada');
    }
    if (contribution.walletAddress.toLowerCase() !== walletAddress.toLowerCase()) {
      throw new ForbiddenException('No tienes permiso para ver esta contribución');
    }
    return { data: this.mapContribution(contribution) };
  }

  async getAudit(address: string) {
    const audit = await this.auditRepository
      .createQueryBuilder('audit')
      .where('LOWER(audit.contractAddress) = LOWER(:address)', { address })
      .getOne();

    if (!audit) {
      return { data: this.defaultAudit(address) };
    }

    return { data: this.mapAudit(audit) };
  }

  async addAuditComment(address: string, author: string, text: string) {
    let audit = await this.auditRepository
      .createQueryBuilder('audit')
      .where('LOWER(audit.contractAddress) = LOWER(:address)', { address })
      .getOne();

    if (!audit) {
      audit = await this.auditRepository.save(this.createDefaultAudit(address));
    }

    const comment = this.auditCommentRepository.create({
      auditId: audit.id,
      author,
      text,
    });
    const saved = await this.auditCommentRepository.save(comment);

    return { data: { id: saved.id, author: saved.author, text: saved.text, createdAt: saved.createdAt } };
  }

  async getWatchlist(userId: string) {
    const items = await this.watchlistRepository.find({ where: { userId } });
    return { data: items.map((item) => item.contractAddress) };
  }

  async addWatchlist(userId: string, contractAddress: string) {
    const existing = await this.watchlistRepository.findOne({
      where: { userId, contractAddress },
    });
    if (!existing) {
      const item = this.watchlistRepository.create({ userId, contractAddress });
      await this.watchlistRepository.save(item);
    }
  }

  async removeWatchlist(userId: string, contractAddress: string) {
    await this.watchlistRepository.delete({ userId, contractAddress });
  }

  async createSubmission(userId: string, payload: any) {
    const submission = this.submissionRepository.create({
      userId,
      contractAddress: payload.contractAddress,
      network: payload.network,
      auditReport: payload.auditReport,
      twitter: payload.twitter,
      telegram: payload.telegram,
      status: SubmissionStatus.PENDING,
    });
    const saved = await this.submissionRepository.save(submission);
    return { data: { id: saved.id, status: saved.status, createdAt: saved.createdAt } };
  }

  async getSubmission(userId: string, id: string) {
    const submission = await this.submissionRepository.findOne({ where: { id, userId } });
    if (!submission) {
      throw new NotFoundException('Submission no encontrada');
    }
    return { data: submission };
  }

  private mapGem(gem: LaunchpadGem) {
    return {
      projectIcon: gem.projectIcon,
      projectName: gem.projectName,
      description: gem.description,
      securityScore: Number(gem.securityScore),
      priceChange: Number(gem.priceChange),
      liquidityDepth: this.formatCurrency(Number(gem.liquidityValue), gem.liquidityCurrency),
      upvotes: this.formatNumber(Number(gem.upvotesNumber)),
      launchDate: gem.launchDate?.toISOString() || null,
      sparklineData: gem.sparklineData || [],
      contractAddress: gem.contractAddress,
      category: gem.category || null,
      isVerified: gem.isVerified,
      rugChecked: gem.rugChecked,
      price: gem.price ? Number(gem.price) : undefined,
      volume24h: gem.volume24h ? Number(gem.volume24h) : undefined,
    };
  }

  private mapFeaturedGem(featured: LaunchpadFeaturedGem) {
    return {
      projectName: featured.projectName,
      subtitle: featured.subtitle,
      description: featured.description,
      endTime: featured.endTime.toISOString(),
      contractAddress: featured.contractAddress,
      projectIcon: featured.projectIcon || null,
      category: featured.category || null,
      raised: featured.raised ? Number(featured.raised) : undefined,
      target: featured.target ? Number(featured.target) : undefined,
      participants: featured.participants || [],
      watchingCount: featured.watchingCount ?? undefined,
      trendingRank: featured.trendingRank ?? undefined,
    };
  }

  private mapPresale(presale: LaunchpadPresale) {
    return {
      id: presale.id,
      projectName: presale.projectName,
      projectDescription: presale.projectDescription,
      projectIcon: presale.projectIcon,
      isVerified: presale.isVerified,
      contractAddress: presale.contractAddress,
      tokenSymbol: presale.tokenSymbol,
      exchangeRate: Number(presale.exchangeRate),
      minBuy: presale.minBuy.toString(),
      maxBuy: presale.maxBuy.toString(),
      endDate: presale.endDate.toISOString(),
      softCap: presale.softCap.toString(),
      hardCap: presale.hardCap.toString(),
      minContrib: presale.minContrib.toString(),
      maxContrib: presale.maxContrib.toString(),
      vestingTerms: presale.vestingTerms,
      auditUrl: presale.auditUrl || null,
      contractUrl: presale.contractUrl || null,
    };
  }

  private mapContribution(contribution: LaunchpadContribution) {
    return {
      id: contribution.id,
      walletAddress: contribution.walletAddress,
      projectName: contribution.projectName,
      projectIcon: contribution.projectIcon,
      tokenSymbol: contribution.tokenSymbol,
      presaleId: contribution.presaleId,
      contribution: `${contribution.contributionAmount.toString()} ETH`,
      buyPrice: `${contribution.buyPrice.toString()} ETH`,
      currentValue: `${contribution.currentValue.toString()} ETH`,
      growth: contribution.growth,
      isLoss: contribution.isLoss,
      vestingProgress: contribution.vestingProgress,
      nextUnlock: contribution.nextUnlock,
      claimableAmount: contribution.claimableAmount,
      status: contribution.status,
      txHash: contribution.txHash || null,
      createdAt: contribution.createdAt.toISOString(),
    };
  }

  private formatCurrency(value: number, currency: string = 'USD') {
    const safeValue = Number.isFinite(value) ? value : 0;
    if (currency === 'USD') {
      return `$${safeValue.toFixed(2)}`;
    }
    return `${safeValue.toFixed(2)} ${currency}`;
  }

  private formatNumber(value: number) {
    return value.toLocaleString('en-US');
  }

  private defaultFeaturedGem() {
    return {
      projectName: 'Coming Soon',
      subtitle: 'Launchpad',
      description: 'No featured gem yet.',
      endTime: new Date().toISOString(),
      contractAddress: '',
      projectIcon: null,
      category: null,
      raised: 0,
      target: 0,
      participants: [],
      watchingCount: 0,
      trendingRank: 0,
    };
  }

  private defaultAudit(address: string) {
    return this.mapAudit(this.createDefaultAudit(address));
  }

  private createDefaultAudit(address: string): LaunchpadAudit {
    return this.auditRepository.create({
      id: randomUUID(),
      projectIcon: '🛡️',
      projectName: 'Unknown Project',
      contractAddress: address,
      fullAddress: address,
      network: 'unknown',
      auditCompleted: new Date().toISOString(),
      isVerified: false,
      verdict: 'Pending',
      riskLevel: 'Unknown',
      trustScore: 0,
      trustSummary: 'No audit data available.',
      securityChecks: [],
      vulnerabilities: { critical: 0, high: 0, medium: 0, low: 0 },
      liquidityLocks: { totalLocked: '0', locks: [] },
      communitySentiment: {
        bullish: 0,
        bearish: 0,
        upvotes: '0',
        watchlists: '0',
        comments: [],
      },
      tokenSymbol: 'N/A',
    });
  }

  private mapAudit(audit: LaunchpadAudit) {
    return {
      projectIcon: audit.projectIcon,
      projectName: audit.projectName,
      contractAddress: audit.contractAddress,
      fullAddress: audit.fullAddress,
      network: audit.network,
      auditCompleted: audit.auditCompleted,
      isVerified: audit.isVerified,
      verdict: audit.verdict,
      riskLevel: audit.riskLevel,
      trustScore: audit.trustScore,
      trustSummary: audit.trustSummary,
      securityChecks: audit.securityChecks || [],
      vulnerabilities: audit.vulnerabilities || { critical: 0, high: 0, medium: 0, low: 0 },
      liquidityLocks: audit.liquidityLocks || { totalLocked: '0', locks: [] },
      communitySentiment: audit.communitySentiment || {
        bullish: 0,
        bearish: 0,
        upvotes: '0',
        watchlists: '0',
        comments: [],
      },
      tokenSymbol: audit.tokenSymbol,
    };
  }

  private async seedDefaults() {
    const gemCount = await this.gemRepository.count();
    if (gemCount === 0) {
      const gem = this.gemRepository.create({
        id: randomUUID(),
        projectIcon: '💎',
        projectName: 'Example Gem',
        description: 'Sample gem for launchpad.',
        securityScore: 85,
        priceChange: 2.4,
        liquidityValue: 2400000,
        liquidityCurrency: 'USD',
        upvotesNumber: 14204,
        launchDate: new Date(),
        sparklineData: [1, 1.1, 1.05, 1.2, 1.15, 1.18, 1.22],
        contractAddress: '0xGEM000000000000000000000000000000000000',
        category: 'DeFi',
        isVerified: true,
        rugChecked: true,
        price: 0.0012,
        volume24h: 820000,
      });
      await this.gemRepository.save(gem);
    }

    const featuredCount = await this.featuredRepository.count();
    if (featuredCount === 0) {
      const featured = this.featuredRepository.create({
        id: randomUUID(),
        projectName: 'Featured Gem',
        subtitle: 'Launchpad Spotlight',
        description: 'Featured presale project.',
        endTime: new Date(Date.now() + 6 * 60 * 60 * 1000),
        contractAddress: '0xFEATURED000000000000000000000000000000000',
        projectIcon: '🚀',
        category: 'Launch',
        raised: 120000,
        target: 500000,
        participants: [],
        watchingCount: 120,
        trendingRank: 1,
      });
      await this.featuredRepository.save(featured);
    }

    const presaleCount = await this.presaleRepository.count();
    if (presaleCount === 0) {
      const presale = this.presaleRepository.create({
        id: randomUUID(),
        projectName: 'Sample Presale',
        projectDescription: 'Sample presale description.',
        projectIcon: '🧪',
        isVerified: false,
        contractAddress: '0xPRESALE000000000000000000000000000000000',
        tokenSymbol: 'SAMP',
        exchangeRate: 100000,
        minBuy: 0.01,
        maxBuy: 2,
        endDate: new Date(Date.now() + 24 * 60 * 60 * 1000),
        softCap: 10,
        hardCap: 100,
        minContrib: 0.01,
        maxContrib: 2,
        vestingTerms: {
          tgeUnlock: '10%',
          cliffPeriod: '1 month',
          linearVesting: '9 months',
          totalMonths: 10,
        },
        auditUrl: null,
        contractUrl: null,
      });
      await this.presaleRepository.save(presale);
    }

    const tokenCount = await this.tokenRepository.count();
    if (tokenCount === 0) {
      const token = this.tokenRepository.create({
        id: randomUUID(),
        projectIcon: '💎',
        projectName: 'Example Gem',
        symbol: 'GEM',
        price: 0.0012,
        priceChange24h: 2.4,
        isVerified: true,
        contractAddress: '0xGEM000000000000000000000000000000000000',
        exchangeRate: 100000,
        sparklineData: [1, 1.1, 1.05, 1.2, 1.15, 1.18, 1.22],
        tokenomics: {
          totalSupply: '1000000000',
          burned: '10000000',
          devWalletLockDays: 180,
        },
        daoSentiment: {
          score: 60,
          label: 'Bullish',
          comments: [],
        },
      });
      await this.tokenRepository.save(token);
    }

    const auditCount = await this.auditRepository.count();
    if (auditCount === 0) {
      const audit = this.createDefaultAudit(
        '0xGEM000000000000000000000000000000000000',
      );
      await this.auditRepository.save(audit);
    }
  }
}
