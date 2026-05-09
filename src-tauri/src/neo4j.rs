// ── NEO4J CLIENT ──────────────────────────────────────────────────────────────
// Handles: signal nodes, stock correlation graph, user behaviour graph

use neo4rs::{query, Graph};
use serde::{Deserialize, Serialize};
use crate::gold::GoldSignal;

#[derive(Clone)]
pub struct Neo4jClient {
    graph: Graph,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CorrelatedStock {
    pub symbol:      String,
    pub strength:    f64,
    pub last_signal: String,
}

impl Neo4jClient {
    pub async fn new(uri: &str, user: &str, password: &str) -> anyhow::Result<Self> {
        let graph = Graph::new(uri, user, password).await?;
        Ok(Self { graph })
    }

    // Run on startup — create indexes for fast lookups
    pub async fn setup_constraints(&self) -> anyhow::Result<()> {
        self.graph.run(query(
            "CREATE CONSTRAINT IF NOT EXISTS FOR (s:Stock) REQUIRE s.symbol IS UNIQUE"
        )).await?;

        self.graph.run(query(
            "CREATE CONSTRAINT IF NOT EXISTS FOR (g:GoldSignal) REQUIRE g.tick_id IS UNIQUE"
        )).await?;

        self.graph.run(query(
            "CREATE CONSTRAINT IF NOT EXISTS FOR (u:User) REQUIRE u.id IS UNIQUE"
        )).await?;

        tracing::info!("Neo4j constraints ready");
        Ok(())
    }

    // Write Gold signal node + link to Stock node
    pub async fn write_signal(&self, gold: &GoldSignal) -> anyhow::Result<()> {
        self.graph.run(
            query("
                MERGE (s:Stock {symbol: $symbol})
                SET   s.last_signal    = $signal,
                      s.last_signal_ts = $timestamp

                CREATE (g:GoldSignal {
                    tick_id:    $tick_id,
                    price:      $price,
                    signal:     $signal,
                    short_ma:   $short_ma,
                    long_ma:    $long_ma,
                    confidence: $confidence,
                    timestamp:  $timestamp
                })
                CREATE (g)-[:FOR_STOCK]->(s)
            ")
            .param("symbol",     gold.symbol.clone())
            .param("tick_id",    gold.tick_id.clone())
            .param("price",      gold.price)
            .param("signal",     gold.signal.clone())
            .param("short_ma",   gold.short_ma)
            .param("long_ma",    gold.long_ma)
            .param("confidence", gold.confidence)
            .param("timestamp",  gold.timestamp as i64),
        ).await?;
        Ok(())
    }

    // Link two stocks with a correlation edge (run periodically, not per tick)
    pub async fn upsert_correlation(
        &self,
        symbol_a: &str,
        symbol_b: &str,
        strength: f64,
    ) -> anyhow::Result<()> {
        self.graph.run(
            query("
                MATCH (a:Stock {symbol: $symbolA})
                MATCH (b:Stock {symbol: $symbolB})
                MERGE (a)-[r:CORRELATED_WITH]-(b)
                SET r.strength   = $strength,
                    r.updated_at = timestamp()
            ")
            .param("symbolA",  symbol_a.to_string())
            .param("symbolB",  symbol_b.to_string())
            .param("strength", strength),
        ).await?;
        Ok(())
    }

    // Get stocks correlated with a given symbol above threshold
    pub async fn correlated_stocks(
        &self,
        symbol: &str,
        min_strength: f64,
    ) -> anyhow::Result<Vec<CorrelatedStock>> {
        let mut result = self.graph.execute(
            query("
                MATCH (s:Stock {symbol: $symbol})
                      -[r:CORRELATED_WITH]-
                      (other:Stock)
                WHERE r.strength >= $min
                RETURN other.symbol     AS symbol,
                       r.strength       AS strength,
                       other.last_signal AS last_signal
                ORDER BY r.strength DESC
                LIMIT 5
            ")
            .param("symbol", symbol.to_string())
            .param("min",    min_strength),
        ).await?;

        let mut stocks = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            stocks.push(CorrelatedStock {
                symbol:      row.get("symbol")?,
                strength:    row.get("strength")?,
                last_signal: row.get::<String>("last_signal").unwrap_or("Hold".into()),
            });
        }
        Ok(stocks)
    }

    // Record that a user acted on a signal (for collaborative filtering later)
    pub async fn record_user_action(
        &self,
        user_id: &str,
        tick_id: &str,
        action:  &str,
    ) -> anyhow::Result<()> {
        self.graph.run(
            query("
                MERGE (u:User {id: $user_id})
                MATCH (g:GoldSignal {tick_id: $tick_id})
                CREATE (u)-[:ACTED_ON {action: $action, at: timestamp()}]->(g)
            ")
            .param("user_id", user_id.to_string())
            .param("tick_id", tick_id.to_string())
            .param("action",  action.to_string()),
        ).await?;
        Ok(())
    }

    // Recommend symbols based on similar users' behaviour
    pub async fn recommend_symbols(&self, user_id: &str) -> anyhow::Result<Vec<String>> {
        let mut result = self.graph.execute(
            query("
                MATCH (u:User {id: $user_id})-[:ACTED_ON]->(:GoldSignal)-[:FOR_STOCK]->(s:Stock)
                WITH u, collect(s.symbol) AS watched

                MATCH (other:User)-[:ACTED_ON]->(:GoldSignal)-[:FOR_STOCK]->(rec:Stock)
                WHERE other <> u AND NOT rec.symbol IN watched

                MATCH (u)-[:ACTED_ON]->(g1:GoldSignal)-[:FOR_STOCK]->(common:Stock)
                      <-[:FOR_STOCK]-(g2:GoldSignal)<-[:ACTED_ON]-(other)

                RETURN rec.symbol AS symbol, count(*) AS score
                ORDER BY score DESC LIMIT 5
            ")
            .param("user_id", user_id.to_string()),
        ).await?;

        let mut symbols = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            symbols.push(row.get::<String>("symbol")?);
        }
        Ok(symbols)
    }
}