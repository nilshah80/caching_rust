//! OpenAPI Documentation
//!
//! Swagger UI and OpenAPI specification endpoints.

use axum::Router;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::http::routes::admin::{
    AclCatRequest,
    AclCatResponse,
    // ACL operations
    AclDelUserRequest,
    AclDelUserResponse,
    AclDryrunRequest,
    AclDryrunResponse,
    AclGenPassRequest,
    AclGenPassResponse,
    AclListResponse,
    AclLoadResponse,
    AclLogRequest,
    AclLogResponse,
    AclSaveResponse,
    AclSetUserRequest,
    AclSetUserResponse,
    AclUsersResponse,
    AclWhoamiResponse,
    ClientGetNameResponse,
    ClientIdResponse,
    ClientInfoResponse,
    ClientKillRequest,
    ClientKillResponse,
    // Client operations
    ClientListResponse,
    ClientPauseRequest,
    ClientPauseResponse,
    ClientSetNameRequest,
    ClientSetNameResponse,
    ClientUnpauseResponse,
    // Command introspection
    CommandCountResponse,
    CommandDocsRequest,
    CommandGetKeysRequest,
    CommandGetKeysResponse,
    CommandInfoRequest,
    CommandListQuery,
    CommandListResponse,
    // Config operations
    ConfigGetRequest,
    ConfigGetResponse,
    ConfigResetStatResponse,
    ConfigRewriteResponse,
    ConfigSetRequest,
    ConfigSetResponse,
    // Database operations
    CopyKeyRequest,
    CopyKeyResponse,
    DbSizeResponse,
    DebugObjectRequest,
    DebugObjectResponse,
    FlushDbRequest,
    // Server info
    LastSaveResponse,
    LatencyDoctorResponse,
    LatencyGraphResponse,
    LatencyHistoryRequest,
    LatencyHistoryResponse,
    LatencyLatestResponse,
    LatencyResetRequest,
    LatencyResetResponse,
    MemoryDoctorResponse,
    MemoryPurgeResponse,
    // Memory operations
    MemoryUsageRequest,
    MoveKeyRequest,
    MoveKeyResponse,
    // Persistence operations
    SaveResponse,
    ShutdownRequest,
    ShutdownResponse,
    // Monitoring operations
    SlowlogGetRequest,
    SlowlogGetResponse,
    SlowlogLenResponse,
    SlowlogResetResponse,
    SwapDbRequest,
    SwapDbResponse,
};
// Domain entities used directly in API responses
use crate::api::http::schemas::bitmaps::{
    BitCountQuery, BitCountResponse, BitGetResponse, BitOpRequest, BitOpResponse, BitOpType,
    BitPosQuery, BitPosResponse, BitSetRequest, BitSetResponse, BitfieldCommandSchema,
    BitfieldEncodingSchema, BitfieldOverflowSchema, BitfieldRequest, BitfieldResponse,
};
use crate::api::http::schemas::bloom::{
    BloomAddRequest, BloomAddResponse, BloomCardResponse, BloomExistsRequest, BloomExistsResponse,
    BloomInfoResponse, BloomInsertRequest, BloomInsertResponse, BloomLoadChunkRequest,
    BloomLoadChunkResponse, BloomReserveRequest, BloomReserveResponse, BloomScanDumpParams,
    BloomScanDumpResponse, CuckooAddRequest, CuckooAddResponse, CuckooCountRequest,
    CuckooCountResponse, CuckooDelRequest, CuckooDelResponse, CuckooExistsRequest,
    CuckooExistsResponse, CuckooInfoResponse, CuckooInsertRequest, CuckooInsertResponse,
    CuckooLoadChunkRequest, CuckooLoadChunkResponse, CuckooReserveRequest, CuckooReserveResponse,
    CuckooScanDumpParams, CuckooScanDumpResponse,
};
use crate::api::http::schemas::common::{KeyInfo, PaginationParams, TtlInfo};
use crate::api::http::schemas::functions::{
    FunctionCallRequest, FunctionCallResponse, FunctionDumpResponse, FunctionFlushModeSchema,
    FunctionFlushRequest, FunctionListQuery, FunctionListResponse, FunctionLoadRequest,
    FunctionLoadResponse, FunctionRestorePolicySchema, FunctionRestoreRequest,
    FunctionStatsResponse, FunctionSuccessResponse,
};
use crate::api::http::schemas::geo::{
    GeoAddRequest, GeoAddResponse, GeoDistQuery, GeoDistResponse, GeoHashRequest, GeoHashResponse,
    GeoMemberSchema, GeoPosRequest, GeoPosResponse, GeoPositionSchema, GeoRadiusByMemberQuery,
    GeoRadiusQuery, GeoSearchCenterSchema, GeoSearchOptionsSchema, GeoSearchRequest,
    GeoSearchResponse, GeoSearchResultItem, GeoSearchShapeSchema, GeoSearchStoreRequest,
    GeoSearchStoreResponse, GeoSortOrderSchema, GeoUnitSchema,
};
use crate::api::http::schemas::hashes::{
    ExpireConditionSchema, GetMultipleFieldsRequest, HExpireAtRequest, HExpireFieldResult,
    HExpireRequest, HExpireResponse, HFieldsRequest, HGetDelRequest, HGetDelResponse,
    HGetExExpirationSchema, HGetExRequest, HGetExResponse, HPExpireAtRequest, HPExpireRequest,
    HSetExConditionSchema, HSetExExpirationSchema, HSetExRequest, HSetExResponse, HashFieldEntry,
    HashIncrFloatRequest, HashIncrRequest, HashRandomFieldResponse, HashScanResponse,
    RandomFieldQuery, ScanHashQuery, SetHashNxRequest, SetHashRequest,
};
use crate::api::http::schemas::json::{
    JsonArrAppendRequest, JsonArrAppendResponse, JsonArrIndexRequest, JsonArrIndexResponse,
    JsonArrInsertRequest, JsonArrInsertResponse, JsonArrLenParams, JsonArrLenResponse,
    JsonArrPopRequest, JsonArrPopResponse, JsonArrTrimRequest, JsonArrTrimResponse,
    JsonClearParams, JsonClearResponse, JsonDebugMemoryParams, JsonDebugMemoryResponse,
    JsonDelParams, JsonDelResponse, JsonGetParams, JsonGetResponse, JsonMGetItem, JsonMGetRequest,
    JsonMGetResponse, JsonMSetItemRequest, JsonMSetRequest, JsonNumIncrByRequest,
    JsonNumMultByRequest, JsonNumResponse, JsonObjKeysParams, JsonObjKeysResponse,
    JsonObjLenParams, JsonObjLenResponse, JsonRespParams, JsonRespResponse, JsonSetRequest,
    JsonSetResponse, JsonStrAppendRequest, JsonStrAppendResponse, JsonStrLenParams,
    JsonStrLenResponse, JsonToggleParams, JsonToggleResponse, JsonTypeParams, JsonTypeResponse,
};
use crate::api::http::schemas::keys::{
    CopyRequest, CopyResponse, DeleteKeysRequest, DeleteKeysResponse, DumpResponse, ExistsRequest,
    ExistsResponse, ExpireRequest, ExpireResponse, KeyInfoResponse, KeysParams, KeysResponse,
    ObjectInfoResponse, PersistResponse, RandomKeyResponse, RenameRequest, RenameResponse,
    RestoreRequest, RestoreResponse, ScanParams, ScanResponse, SortOrderSchema, SortRequest,
    SortResponse, SortStoreRequest, SortStoreResponse, TouchRequest, TouchResponse, TtlResponse,
    TypeResponse,
};
use crate::api::http::schemas::lists::{
    BLMPopRequest, BLMPopStreamQuery, BlockingMoveRequest, BlockingPopRequest, BlockingPopResponse,
    BlockingPopStreamQuery, InsertPositionParam, LMPopRequest, LMPopResponse, ListDirectionParam,
    ListIndexQuery, ListIndexResponse, ListInsertRequest, ListInsertResponse, ListLengthResponse,
    ListMoveRequest, ListMoveResponse, ListPopRequest, ListPopResponse, ListPosQuery,
    ListPosResponse, ListPushRequest, ListPushResponse, ListRangeQuery, ListRemoveRequest,
    ListRemoveResponse, ListSetRequest, ListTrimRequest,
};
use crate::api::http::schemas::probabilistic::{
    CmsIncrByItem, CmsIncrByRequest, CmsIncrByResponse, CmsInfoResponse, CmsInitByDimRequest,
    CmsInitByProbRequest, CmsInitResponse, CmsMergeRequest, CmsMergeResponse, CmsQueryRequest,
    CmsQueryResponse, PfAddRequest, PfAddResponse, PfCountRequest, PfCountResponse, PfMergeRequest,
    PfMergeResponse, TDigestAckResponse, TDigestAddRequest, TDigestCreateRequest,
    TDigestInfoResponse, TDigestMergeRequest, TDigestQuantileRequest, TDigestRanksRequest,
    TDigestRanksResponse, TDigestScalarResponse, TDigestTrimmedMeanRequest, TDigestValuesRequest,
    TDigestValuesResponse, TopKAddRequest, TopKAddResponse, TopKCountResponse, TopKIncrByItem,
    TopKIncrByRequest, TopKIncrByResponse, TopKInfoResponse, TopKListItem, TopKListQuery,
    TopKListResponse, TopKQueryRequest, TopKQueryResponse, TopKReserveRequest, TopKReserveResponse,
};
use crate::api::http::schemas::pubsub::{
    ChannelsResponse, NumPatResponse, NumSubItem, NumSubRequest, NumSubResponse, PubSubMessage,
    PubSubStatsResponse, PublishRequest, PublishResponse, SubscriptionConfirmation, WebSocketError,
};
use crate::api::http::schemas::scripting::{
    EvalRequest, EvalResponse, EvalShaRequest, FlushMode, ScriptDebugMode, ScriptDebugRequest,
    ScriptDebugResponse, ScriptExistsRequest, ScriptExistsResponse, ScriptExistsResult,
    ScriptFlushRequest, ScriptFlushResponse, ScriptKillResponse, ScriptLoadRequest,
    ScriptLoadResponse,
};
use crate::api::http::schemas::search::{
    AggregateOptionsDto, AggregateRequest, AggregateResponse, AliasRequest, AliasResponse,
    AlterIndexRequest, AlterIndexResponse, CreateIndexRequest, CreateIndexResponse,
    CursorDelResponse, CursorReadParams, CursorReadResponse, DictDumpResponse, DictResponse,
    DictTermsRequest, DropIndexParams, DropIndexResponse, ExplainRequest, ExplainResponse,
    HybridSearchRequest, HybridSearchResponse, IndexCreateOptionsDto, IndexInfoResponse,
    ListIndicesResponse, ProfileRequest, ProfileResponse, SearchConfigGetResponse,
    SearchConfigSetRequest, SearchConfigSetResponse, SearchFieldSchemaDto, SearchOptionsDto,
    SearchRequest, SearchResponse, SpellcheckRequest, SpellcheckResponse, SugAddRequest,
    SugAddResponse, SugDelRequest, SugDelResponse, SugGetParams, SugGetResponse, SugLenResponse,
    SynonymDumpResponse, SynonymUpdateRequest, SynonymUpdateResponse,
};
use crate::api::http::schemas::sets::{
    SetAddRequest, SetAddResponse, SetAlgebraRequest, SetAlgebraResponse, SetAlgebraStoreRequest,
    SetAlgebraStoreResponse, SetCardResponse, SetInterCardRequest, SetInterCardResponse,
    SetIsMemberRequest, SetIsMemberResponse, SetMIsMemberRequest, SetMIsMemberResponse,
    SetMembersResponse, SetMoveRequest, SetMoveResponse, SetPopRequest, SetPopResponse,
    SetRandMemberQuery, SetRandMemberResponse, SetRemoveRequest, SetRemoveResponse, SetScanQuery,
    SetScanResponse,
};
use crate::api::http::schemas::sorted_sets::{
    LexRangeDto, ScoreRangeDto, ScoredMemberDto, ZAddIncrRequest, ZAddIncrResponse, ZAddOptionsDto,
    ZAddRequest, ZAddResponse, ZAggregateDto, ZBMPopRequest, ZBPopRequest, ZBPopResponse,
    ZBPopStreamQuery, ZCardResponse, ZCountRequest, ZCountResponse, ZDiffRequest,
    ZDiffStoreRequest, ZIncrByRequest, ZIncrByResponse, ZInterCardRequest, ZInterCardResponse,
    ZLexCountRequest, ZMPopRequest, ZMPopResponse, ZMScoreRequest, ZMScoreResponse, ZPopQuery,
    ZPopResponse, ZRandMemberQuery, ZRandMemberResponse, ZRangeByLexRequest, ZRangeByLexResponse,
    ZRangeByScoreRequest, ZRangeQuery, ZRangeResponse, ZRangeStoreRequest, ZRangeStoreResponse,
    ZRankResponse, ZRemRangeByLexRequest, ZRemRangeByRankRequest, ZRemRangeByScoreRequest,
    ZRemRangeResponse, ZRemRequest, ZRemResponse, ZScanQuery, ZScanResponse, ZScoreResponse,
    ZSetAlgebraOptionsDto, ZSetAlgebraRequest, ZSetAlgebraResponse, ZSetAlgebraStoreRequest,
    ZSetAlgebraStoreResponse,
};
use crate::api::http::schemas::streams::{
    ConsumerCreateRequest, ConsumerGroupCreateRequest, ConsumerGroupCreateResponse,
    ConsumerGroupSetIdRequest, ConsumerOperationResponse, PendingQuery, StreamAckRequest,
    StreamAckResponse, StreamAddRequest, StreamAddResponse, StreamAutoClaimRequest,
    StreamClaimRequest, StreamDeleteRequest, StreamDeleteResponse, StreamEntriesResponse,
    StreamGroupSubscribeQuery, StreamIdPair, StreamInfoQuery, StreamLengthResponse,
    StreamRangeQuery, StreamReadBlockingRequest, StreamReadGroupBlockingRequest,
    StreamReadGroupRequest, StreamReadRequest, StreamSetIdRequest, StreamSubscribeQuery,
    StreamTrimRequest, StreamTrimResponse, TrimStrategyParam,
};
use crate::api::http::schemas::strings::{
    AppendRequest, AppendResponse, GetDelResponse, GetExParams, GetRangeParams, GetRangeResponse,
    IncrementRequest, IncrementResponse, LcsMatchSchema, LcsRequest, LcsResponse, MGetRequest,
    MGetResponse, MSetRequest, MSetResponse, SetRangeRequest, SetRangeResponse, SetStringRequest,
    SetStringResponse, StrLenResponse,
};
use crate::api::http::schemas::timeseries::{
    Aggregation, DuplicatePolicy, Sample, TimeSeriesAddRequest, TimeSeriesCreateRequest,
    TimeSeriesGetResponse, TimeSeriesMGetItem, TimeSeriesMGetRequest, TimeSeriesMGetResponse,
    TimeSeriesMRangeRequest, TimeSeriesMRangeResponse, TimeSeriesRangeItem, TimeSeriesRangeQuery,
    TimeSeriesRangeResponse, TimeSeriesWriteResponse, TsAlterRequest, TsCreateRuleRequest,
    TsDelQuery, TsDelResponse, TsIncrDecrRequest, TsInfoResponse, TsMaddItem, TsMaddRequest,
    TsMaddResponse, TsMrevRangeRequest, TsQueryIndexRequest, TsQueryIndexResponse,
};
use crate::api::http::schemas::transactions::{
    CommandResult, CompareAndSetRequest, CompareAndSetResponse, FieldValue, HCompareAndSetRequest,
    KeyValue, RedisCommand, ScoredMember as TransactionScoredMember, TransactionRequest,
    TransactionResponse,
};
use crate::api::http::schemas::vectors::{
    VectorAddRequest, VectorAddResponse, VectorCardResponse, VectorDimResponse, VectorEmbRequest,
    VectorEmbResponse, VectorGetAttrResponse, VectorInfoResponse, VectorIsMemberRequest,
    VectorIsMemberResponse, VectorLinksLayer, VectorLinksResponse, VectorRandMemberRequest,
    VectorRandMemberResponse, VectorRangeItemResponse, VectorRangeRequest, VectorRangeResponse,
    VectorRemRequest, VectorRemResponse, VectorSetAttrRequest, VectorSetAttrResponse,
    VectorSimItemResponse, VectorSimRequest, VectorSimResponse,
};
use crate::domain::entities::{
    AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, FlushResult, LatencyEvent,
    MemoryStats, MemoryUsage, ServerInfo, ServerTime, SlowlogEntry,
};
use crate::domain::entities::{
    // Search entities
    AggregateOptions,
    AggregateResult,
    AggregateStep,
    ApplyStep,
    AutoClaimResult,
    ClaimResult,
    ConsumerGroupInfo,
    ConsumerInfo,
    DistanceMetric,
    FilterStep,
    GeoFilter,
    GroupByStep,
    HighlightOptions,
    IndexCreateOptions,
    IndexDataType,
    IndexInfo,
    LimitStep,
    NumericFilter,
    PendingEntry,
    PendingSummary,
    PhoneticMatcher,
    ProfileType,
    Reducer,
    SearchDocument,
    SearchFieldSchema,
    SearchFieldType,
    SearchOptions,
    SearchResult,
    SortBy,
    SortByStep,
    SortOrder,
    SpellcheckResult,
    SpellcheckSuggestion,
    SpellcheckTerm,
    StreamEntry,
    StreamInfo,
    StreamReadResult,
    StringValue,
    SugAddOptions,
    SugGetOptions,
    Suggestion,
    SummarizeOptions,
    SynonymGroup,
    VectorAlgorithm,
    VectorFieldOptions,
};
use crate::domain::errors::{ErrorDetail, ErrorResponse};
use crate::infrastructure::redis::capabilities::{
    FeatureCapabilities, ModuleCapabilities, RedisCapabilities,
};
use crate::infrastructure::redis::connection::PoolStats;
use crate::shared::app_state::AppState;

/// OpenAPI documentation for the Redis Caching Service
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Redis Caching Service API",
        version = "0.1.0",
        description = "A high-performance Redis caching service with comprehensive Redis operations through a clean REST API",
        license(name = "MIT"),
        contact(name = "API Support")
    ),
    servers(
        (url = "/", description = "Local server")
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Keys", description = "Redis key management operations (DELETE, EXISTS, EXPIRE, TTL, RENAME, COPY, SCAN, etc.)"),
        (name = "Strings", description = "Redis string operations (GET, SET, MGET, MSET, INCR, etc.)"),
        (name = "Hashes", description = "Redis hash operations (HGET, HSET, HMGET, HGETALL, HINCRBY, etc.)"),
        (name = "Lists", description = "Redis list operations (LPUSH, RPUSH, LPOP, RPOP, LRANGE, LLEN, LINDEX, LINSERT, BLPOP, BRPOP, etc.)"),
        (name = "Sets", description = "Redis set operations (SADD, SREM, SMEMBERS, SISMEMBER, SCARD, SINTER, SUNION, SDIFF, SPOP, SRANDMEMBER, etc.)"),
        (name = "Sorted Sets", description = "Redis sorted set operations (ZADD, ZREM, ZSCORE, ZRANK, ZRANGE, ZCOUNT, ZPOPMIN, ZPOPMAX, BZPOPMIN, BZPOPMAX, ZUNION, ZINTER, ZDIFF, ZSCAN, etc.)"),
        (name = "Bitmaps", description = "Redis bitmap operations (SETBIT, GETBIT, BITCOUNT, BITPOS, BITOP, BITFIELD, BITFIELD_RO) - core Redis feature"),
        (name = "Streams", description = "Redis stream operations (XADD, XREAD, XRANGE, XLEN, XTRIM, consumer groups, etc.)"),
        (name = "Streams (Admin)", description = "Redis stream admin operations (XGROUP CREATE/DESTROY, XSETID, consumer management)"),
        (name = "JSON", description = "RedisJSON operations (JSON.SET, JSON.GET, JSON.DEL, JSON.MGET, array operations, object operations, numeric operations, etc.) - requires RedisJSON module"),
        (name = "Search", description = "RediSearch operations (FT.CREATE, FT.SEARCH, FT.AGGREGATE, FT.INFO, autocomplete, synonyms, spellcheck, etc.) - requires RediSearch module"),
        (name = "Bloom Filters", description = "RedisBloom Bloom filter operations (BF.RESERVE, BF.ADD, BF.EXISTS, BF.INFO, BF.SCANDUMP, BF.LOADCHUNK, etc.) - requires RedisBloom module"),
        (name = "Cuckoo Filters", description = "RedisBloom Cuckoo filter operations (CF.RESERVE, CF.ADD, CF.ADDNX, CF.EXISTS, CF.DEL, CF.INFO, CF.SCANDUMP, CF.LOADCHUNK, etc.) - requires RedisBloom module"),
        (name = "Count-Min Sketch", description = "RedisBloom Count-Min Sketch operations for frequency estimation (CMS.INITBYDIM, CMS.INITBYPROB, CMS.INCRBY, CMS.QUERY, CMS.MERGE, CMS.INFO) - requires RedisBloom module"),
        (name = "Top-K", description = "RedisBloom Top-K operations for tracking most frequent items (TOPK.RESERVE, TOPK.ADD, TOPK.INCRBY, TOPK.QUERY, TOPK.COUNT, TOPK.LIST, TOPK.INFO) - requires RedisBloom module"),
        (name = "T-Digest", description = "RedisBloom T-Digest operations for quantile estimation (TDIGEST.CREATE, TDIGEST.ADD, TDIGEST.QUANTILE, TDIGEST.CDF, TDIGEST.RANK, TDIGEST.REVRANK, TDIGEST.BYRANK, TDIGEST.BYREVRANK, TDIGEST.MIN, TDIGEST.MAX, TDIGEST.INFO, TDIGEST.MERGE, TDIGEST.RESET, TDIGEST.TRIMMED_MEAN) - requires RedisBloom module"),
        (name = "HyperLogLog", description = "Redis HyperLogLog operations for cardinality estimation (PFADD, PFCOUNT, PFMERGE) - core Redis feature"),
        (name = "Geo", description = "Redis geospatial operations (GEOADD, GEODIST, GEOHASH, GEOPOS, GEOSEARCH, GEOSEARCHSTORE, GEORADIUS, GEORADIUSBYMEMBER) - core Redis feature since 3.2"),
        (name = "Pub/Sub", description = "Redis Pub/Sub operations (PUBLISH, SUBSCRIBE, PSUBSCRIBE, PUBSUB CHANNELS/NUMSUB/NUMPAT). HTTP endpoints for publish and info commands, WebSocket endpoints for subscriptions - core Redis feature"),
        (name = "Transactions", description = "Redis transaction operations (MULTI/EXEC bundled in single request, compare-and-set operations using Lua scripts). Supports WATCH for optimistic locking - core Redis feature"),
        (name = "Scripting", description = "Redis Lua scripting operations (EVAL, EVALSHA, SCRIPT LOAD/EXISTS/FLUSH/KILL/DEBUG). Execute custom Lua scripts with keys and arguments, manage script cache - core Redis feature"),
        (name = "Vectors", description = "Redis Vector Sets operations (VADD, VREM, VSIM, VCARD, VDIM, VEMB, VISMEMBER, VLINKS, VRANDMEMBER, VRANGE, VINFO, VGETATTR, VSETATTR) - requires Redis 8.0+"),
        (name = "Admin", description = "Administrative endpoints (pool stats, capabilities, server info, database ops, config, persistence, client management, monitoring, ACL)")
    ),
    paths(
        // Health endpoints
        crate::api::http::routes::health::health,
        crate::api::http::routes::health::readiness,
        crate::api::http::routes::health::liveness,
        // Key endpoints
        crate::api::http::routes::keys::delete_keys,
        crate::api::http::routes::keys::exists_keys,
        crate::api::http::routes::keys::touch_keys,
        crate::api::http::routes::keys::scan_keys,
        crate::api::http::routes::keys::list_keys,
        crate::api::http::routes::keys::random_key,
        crate::api::http::routes::keys::get_key_info,
        crate::api::http::routes::keys::delete_single_key,
        crate::api::http::routes::keys::get_ttl,
        crate::api::http::routes::keys::set_expire,
        crate::api::http::routes::keys::persist_key,
        crate::api::http::routes::keys::get_type,
        crate::api::http::routes::keys::rename_key,
        crate::api::http::routes::keys::copy_key,
        crate::api::http::routes::keys::dump_key,
        crate::api::http::routes::keys::restore_key,
        crate::api::http::routes::keys::get_object_info,
        crate::api::http::routes::keys::sort_key,
        crate::api::http::routes::keys::sort_store_key,
        crate::api::http::routes::keys::sort_ro_key,
        // String endpoints
        crate::api::http::routes::strings::get_string,
        crate::api::http::routes::strings::set_string,
        crate::api::http::routes::strings::get_del_string,
        crate::api::http::routes::strings::mget_strings,
        crate::api::http::routes::strings::mset_strings,
        crate::api::http::routes::strings::incr_string,
        crate::api::http::routes::strings::decr_string,
        crate::api::http::routes::strings::append_string,
        crate::api::http::routes::strings::strlen_string,
        crate::api::http::routes::strings::get_range,
        crate::api::http::routes::strings::set_range,
        crate::api::http::routes::strings::get_ex_string,
        crate::api::http::routes::strings::lcs,
        // Hash endpoints
        crate::api::http::routes::hashes::hget,
        crate::api::http::routes::hashes::hset,
        crate::api::http::routes::hashes::hset_nx,
        crate::api::http::routes::hashes::hgetall,
        crate::api::http::routes::hashes::hmget,
        crate::api::http::routes::hashes::hdel,
        crate::api::http::routes::hashes::hexists,
        crate::api::http::routes::hashes::hkeys,
        crate::api::http::routes::hashes::hvals,
        crate::api::http::routes::hashes::hlen,
        crate::api::http::routes::hashes::hincr_by,
        crate::api::http::routes::hashes::hincr_by_float,
        crate::api::http::routes::hashes::hstr_len,
        crate::api::http::routes::hashes::hrand_field,
        crate::api::http::routes::hashes::hscan,
        // Hash field expiration endpoints (Redis 7.4+)
        crate::api::http::routes::hashes::hexpire,
        crate::api::http::routes::hashes::hpexpire,
        crate::api::http::routes::hashes::hexpire_at,
        crate::api::http::routes::hashes::hpexpire_at,
        crate::api::http::routes::hashes::hexpire_time,
        crate::api::http::routes::hashes::hpexpire_time,
        crate::api::http::routes::hashes::httl,
        crate::api::http::routes::hashes::hpttl,
        crate::api::http::routes::hashes::hpersist,
        // Redis 8.0+ hash endpoints
        crate::api::http::routes::hashes::hgetex,
        crate::api::http::routes::hashes::hsetex,
        crate::api::http::routes::hashes::hgetdel,
        // List endpoints
        crate::api::http::routes::lists::lpush,
        crate::api::http::routes::lists::rpush,
        crate::api::http::routes::lists::lpush_x,
        crate::api::http::routes::lists::rpush_x,
        crate::api::http::routes::lists::lpop,
        crate::api::http::routes::lists::rpop,
        crate::api::http::routes::lists::lrange,
        crate::api::http::routes::lists::llen,
        crate::api::http::routes::lists::lindex,
        crate::api::http::routes::lists::lset,
        crate::api::http::routes::lists::linsert,
        crate::api::http::routes::lists::lrem,
        crate::api::http::routes::lists::ltrim,
        crate::api::http::routes::lists::lpos,
        crate::api::http::routes::lists::lmove,
        crate::api::http::routes::lists::rpop_lpush,
        crate::api::http::routes::lists::blpop,
        crate::api::http::routes::lists::brpop,
        crate::api::http::routes::lists::blmove,
        crate::api::http::routes::lists::brpop_lpush,
        crate::api::http::routes::lists::lmpop,
        crate::api::http::routes::lists::blmpop,
        crate::api::http::routes::lists::blpop_stream,
        crate::api::http::routes::lists::brpop_stream,
        crate::api::http::routes::lists::blmpop_stream,
        // Set endpoints
        crate::api::http::routes::sets::sadd,
        crate::api::http::routes::sets::srem,
        crate::api::http::routes::sets::smembers,
        crate::api::http::routes::sets::sismember,
        crate::api::http::routes::sets::smismember,
        crate::api::http::routes::sets::scard,
        crate::api::http::routes::sets::srandmember,
        crate::api::http::routes::sets::spop,
        crate::api::http::routes::sets::smove,
        crate::api::http::routes::sets::sinter,
        crate::api::http::routes::sets::sinterstore,
        crate::api::http::routes::sets::sintercard,
        crate::api::http::routes::sets::sunion,
        crate::api::http::routes::sets::sunionstore,
        crate::api::http::routes::sets::sdiff,
        crate::api::http::routes::sets::sdiffstore,
        crate::api::http::routes::sets::sscan,
        // Sorted Set endpoints
        crate::api::http::routes::sorted_sets::zadd,
        crate::api::http::routes::sorted_sets::zadd_incr,
        crate::api::http::routes::sorted_sets::zrem,
        crate::api::http::routes::sorted_sets::zscore,
        crate::api::http::routes::sorted_sets::zmscore,
        crate::api::http::routes::sorted_sets::zincrby,
        crate::api::http::routes::sorted_sets::zcard,
        crate::api::http::routes::sorted_sets::zcount,
        crate::api::http::routes::sorted_sets::zlexcount,
        crate::api::http::routes::sorted_sets::zrank,
        crate::api::http::routes::sorted_sets::zrevrank,
        crate::api::http::routes::sorted_sets::zrange,
        crate::api::http::routes::sorted_sets::zrangebyscore,
        crate::api::http::routes::sorted_sets::zrangebylex,
        crate::api::http::routes::sorted_sets::zrangestore,
        crate::api::http::routes::sorted_sets::zremrangebyrank,
        crate::api::http::routes::sorted_sets::zremrangebyscore,
        crate::api::http::routes::sorted_sets::zremrangebylex,
        crate::api::http::routes::sorted_sets::zpopmin,
        crate::api::http::routes::sorted_sets::zpopmax,
        crate::api::http::routes::sorted_sets::bzpopmin,
        crate::api::http::routes::sorted_sets::bzpopmax,
        crate::api::http::routes::sorted_sets::zmpop,
        crate::api::http::routes::sorted_sets::bzmpop,
        crate::api::http::routes::sorted_sets::zrandmember,
        crate::api::http::routes::sorted_sets::zunion,
        crate::api::http::routes::sorted_sets::zunionstore,
        crate::api::http::routes::sorted_sets::zinter,
        crate::api::http::routes::sorted_sets::zinterstore,
        crate::api::http::routes::sorted_sets::zintercard,
        crate::api::http::routes::sorted_sets::zdiff,
        crate::api::http::routes::sorted_sets::zdiffstore,
        crate::api::http::routes::sorted_sets::zscan,
        crate::api::http::routes::sorted_sets::bzpopmin_stream,
        crate::api::http::routes::sorted_sets::bzpopmax_stream,
        // Bitmap endpoints (core Redis)
        crate::api::http::routes::bitmaps::getbit,
        crate::api::http::routes::bitmaps::setbit,
        crate::api::http::routes::bitmaps::bitcount,
        crate::api::http::routes::bitmaps::bitpos,
        crate::api::http::routes::bitmaps::bitop,
        crate::api::http::routes::bitmaps::bitfield,
        crate::api::http::routes::bitmaps::bitfield_ro,
        // Stream endpoints
        crate::api::http::routes::streams::xadd,
        crate::api::http::routes::streams::xlen,
        crate::api::http::routes::streams::xrange,
        crate::api::http::routes::streams::xrevrange,
        crate::api::http::routes::streams::xdel,
        crate::api::http::routes::streams::xtrim,
        crate::api::http::routes::streams::xinfo_stream,
        crate::api::http::routes::streams::xread,
        crate::api::http::routes::streams::xread_blocking,
        crate::api::http::routes::streams::stream_subscribe,
        crate::api::http::routes::streams::xinfo_groups,
        crate::api::http::routes::streams::xinfo_consumers,
        crate::api::http::routes::streams::xreadgroup,
        crate::api::http::routes::streams::xreadgroup_blocking,
        crate::api::http::routes::streams::xack,
        crate::api::http::routes::streams::xpending_summary,
        crate::api::http::routes::streams::xpending,
        crate::api::http::routes::streams::xclaim,
        crate::api::http::routes::streams::xautoclaim,
        crate::api::http::routes::streams::stream_group_subscribe,
        // Stream admin endpoints
        crate::api::http::routes::streams::xgroup_create,
        crate::api::http::routes::streams::xgroup_destroy,
        crate::api::http::routes::streams::xgroup_setid,
        crate::api::http::routes::streams::xgroup_createconsumer,
        crate::api::http::routes::streams::xgroup_delconsumer,
        crate::api::http::routes::streams::xsetid,
        // JSON endpoints (RedisJSON module)
        crate::api::http::routes::json::json_set,
        crate::api::http::routes::json::json_get,
        crate::api::http::routes::json::json_del,
        crate::api::http::routes::json::json_mget,
        crate::api::http::routes::json::json_mset,
        crate::api::http::routes::json::json_type,
        crate::api::http::routes::json::json_str_len,
        crate::api::http::routes::json::json_str_append,
        crate::api::http::routes::json::json_num_incr_by,
        crate::api::http::routes::json::json_num_mult_by,
        crate::api::http::routes::json::json_toggle,
        crate::api::http::routes::json::json_clear,
        crate::api::http::routes::json::json_arr_len,
        crate::api::http::routes::json::json_arr_append,
        crate::api::http::routes::json::json_arr_index,
        crate::api::http::routes::json::json_arr_insert,
        crate::api::http::routes::json::json_arr_pop,
        crate::api::http::routes::json::json_arr_trim,
        crate::api::http::routes::json::json_obj_len,
        crate::api::http::routes::json::json_obj_keys,
        crate::api::http::routes::json::json_debug_memory,
        crate::api::http::routes::json::json_resp,
        // Search endpoints (RediSearch module)
        crate::api::http::routes::search::create_index,
        crate::api::http::routes::search::list_indices,
        crate::api::http::routes::search::get_index_info,
        crate::api::http::routes::search::drop_index,
        crate::api::http::routes::search::alter_index,
        crate::api::http::routes::search::search,
        crate::api::http::routes::search::aggregate,
        crate::api::http::routes::search::hybrid_search,
        crate::api::http::routes::search::explain,
        crate::api::http::routes::search::profile,
        crate::api::http::routes::search::alias_add,
        crate::api::http::routes::search::alias_del,
        crate::api::http::routes::search::alias_update,
        crate::api::http::routes::search::sug_add,
        crate::api::http::routes::search::sug_get,
        crate::api::http::routes::search::sug_del,
        crate::api::http::routes::search::sug_len,
        crate::api::http::routes::search::syn_dump,
        crate::api::http::routes::search::syn_update,
        crate::api::http::routes::search::spellcheck,
        crate::api::http::routes::search::dict_add,
        crate::api::http::routes::search::dict_del,
        crate::api::http::routes::search::dict_dump,
        crate::api::http::routes::search::config_get,
        crate::api::http::routes::search::config_set,
        crate::api::http::routes::search::cursor_read,
        crate::api::http::routes::search::cursor_del,
        // Bloom filter endpoints (RedisBloom module)
        crate::api::http::routes::bloom::bf_reserve,
        crate::api::http::routes::bloom::bf_info,
        crate::api::http::routes::bloom::bf_add,
        crate::api::http::routes::bloom::bf_exists,
        crate::api::http::routes::bloom::bf_insert,
        crate::api::http::routes::bloom::bf_card,
        crate::api::http::routes::bloom::bf_scandump,
        crate::api::http::routes::bloom::bf_loadchunk,
        // Cuckoo filter endpoints (RedisBloom module)
        crate::api::http::routes::bloom::cf_reserve,
        crate::api::http::routes::bloom::cf_info,
        crate::api::http::routes::bloom::cf_add,
        crate::api::http::routes::bloom::cf_addnx,
        crate::api::http::routes::bloom::cf_exists,
        crate::api::http::routes::bloom::cf_insert,
        crate::api::http::routes::bloom::cf_insertnx,
        crate::api::http::routes::bloom::cf_del,
        crate::api::http::routes::bloom::cf_count,
        crate::api::http::routes::bloom::cf_scandump,
        crate::api::http::routes::bloom::cf_loadchunk,
        // Count-Min Sketch endpoints (RedisBloom module)
        crate::api::http::routes::probabilistic::cms_init_by_dim,
        crate::api::http::routes::probabilistic::cms_init_by_prob,
        crate::api::http::routes::probabilistic::cms_incr_by,
        crate::api::http::routes::probabilistic::cms_query,
        crate::api::http::routes::probabilistic::cms_merge,
        crate::api::http::routes::probabilistic::cms_info,
        // Top-K endpoints (RedisBloom module)
        crate::api::http::routes::probabilistic::topk_reserve,
        crate::api::http::routes::probabilistic::topk_info,
        crate::api::http::routes::probabilistic::topk_add,
        crate::api::http::routes::probabilistic::topk_incr_by,
        crate::api::http::routes::probabilistic::topk_query,
        crate::api::http::routes::probabilistic::topk_count,
        crate::api::http::routes::probabilistic::topk_list,
        // T-Digest endpoints (RedisBloom module)
        crate::api::http::routes::probabilistic::tdigest_create,
        crate::api::http::routes::probabilistic::tdigest_info,
        crate::api::http::routes::probabilistic::tdigest_add,
        crate::api::http::routes::probabilistic::tdigest_quantile,
        crate::api::http::routes::probabilistic::tdigest_cdf,
        crate::api::http::routes::probabilistic::tdigest_rank,
        crate::api::http::routes::probabilistic::tdigest_revrank,
        crate::api::http::routes::probabilistic::tdigest_byrank,
        crate::api::http::routes::probabilistic::tdigest_byrevrank,
        crate::api::http::routes::probabilistic::tdigest_min,
        crate::api::http::routes::probabilistic::tdigest_max,
        crate::api::http::routes::probabilistic::tdigest_merge,
        crate::api::http::routes::probabilistic::tdigest_reset,
        crate::api::http::routes::probabilistic::tdigest_trimmed_mean,
        // HyperLogLog endpoints (core Redis)
        crate::api::http::routes::probabilistic::pf_add,
        crate::api::http::routes::probabilistic::pf_count,
        crate::api::http::routes::probabilistic::pf_merge,
        // Geo endpoints (core Redis)
        crate::api::http::routes::geo::geo_add,
        crate::api::http::routes::geo::geo_pos,
        crate::api::http::routes::geo::geo_dist,
        crate::api::http::routes::geo::geo_hash,
        crate::api::http::routes::geo::geo_search,
        crate::api::http::routes::geo::geo_search_store,
        crate::api::http::routes::geo::geo_radius,
        crate::api::http::routes::geo::geo_radius_by_member,
        // Pub/Sub endpoints (core Redis)
        crate::api::http::routes::pubsub::publish,
        crate::api::http::routes::pubsub::channels,
        crate::api::http::routes::pubsub::numsub,
        crate::api::http::routes::pubsub::numpat,
        crate::api::http::routes::pubsub::stats,
        crate::api::http::routes::pubsub::ws_subscribe,
        crate::api::http::routes::pubsub::ws_psubscribe,
        // Sharded Pub/Sub endpoints (Redis 7.0+ cluster)
        crate::api::http::routes::pubsub::spublish,
        crate::api::http::routes::pubsub::shardchannels,
        crate::api::http::routes::pubsub::shardnumsub,
        crate::api::http::routes::pubsub::ws_ssubscribe,
        // Transaction endpoints (core Redis)
        crate::api::http::routes::transactions::execute,
        crate::api::http::routes::transactions::compare_and_set,
        crate::api::http::routes::transactions::hcompare_and_set,
        // Scripting endpoints (core Redis)
        crate::api::http::routes::scripting::eval,
        crate::api::http::routes::scripting::evalsha,
        crate::api::http::routes::scripting::script_load,
        crate::api::http::routes::scripting::script_exists,
        crate::api::http::routes::scripting::script_flush,
        crate::api::http::routes::scripting::script_kill,
        crate::api::http::routes::scripting::script_debug,
        // Redis Functions endpoints
        crate::api::http::routes::functions::function_list,
        crate::api::http::routes::functions::function_load,
        crate::api::http::routes::functions::function_delete,
        crate::api::http::routes::functions::function_flush,
        crate::api::http::routes::functions::function_call,
        crate::api::http::routes::functions::function_dump,
        crate::api::http::routes::functions::function_restore,
        crate::api::http::routes::functions::function_stats,
        crate::api::http::routes::functions::function_kill,
        // Admin - Public endpoints
        crate::api::http::routes::admin::get_pool_stats,
        crate::api::http::routes::admin::get_capabilities,
        // Admin - Server info
        crate::api::http::routes::admin::get_server_info,
        crate::api::http::routes::admin::get_server_time,
        crate::api::http::routes::admin::get_db_size,
        crate::api::http::routes::admin::get_lastsave,
        crate::api::http::routes::admin::debug_object,
        crate::api::http::routes::admin::shutdown,
        // Admin - Memory
        crate::api::http::routes::admin::get_memory_stats,
        crate::api::http::routes::admin::get_memory_usage,
        crate::api::http::routes::admin::memory_doctor,
        crate::api::http::routes::admin::memory_purge,
        // Admin - Database operations
        crate::api::http::routes::admin::flush_db,
        crate::api::http::routes::admin::flush_all,
        crate::api::http::routes::admin::copy_key,
        crate::api::http::routes::admin::move_key,
        crate::api::http::routes::admin::swap_db,
        // Admin - Config
        crate::api::http::routes::admin::config_get,
        crate::api::http::routes::admin::config_set,
        crate::api::http::routes::admin::config_rewrite,
        crate::api::http::routes::admin::config_resetstat,
        // Admin - Persistence
        crate::api::http::routes::admin::save,
        crate::api::http::routes::admin::bgsave,
        crate::api::http::routes::admin::bgrewriteaof,
        // Admin - Client operations
        crate::api::http::routes::admin::client_list,
        crate::api::http::routes::admin::client_kill,
        crate::api::http::routes::admin::client_pause,
        crate::api::http::routes::admin::client_unpause,
        crate::api::http::routes::admin::client_setname,
        crate::api::http::routes::admin::client_getname,
        crate::api::http::routes::admin::client_id,
        crate::api::http::routes::admin::client_info,
        // Admin - Slowlog
        crate::api::http::routes::admin::slowlog_get,
        crate::api::http::routes::admin::slowlog_len,
        crate::api::http::routes::admin::slowlog_reset,
        // Admin - Latency
        crate::api::http::routes::admin::latency_latest,
        crate::api::http::routes::admin::latency_history,
        crate::api::http::routes::admin::latency_doctor,
        crate::api::http::routes::admin::latency_reset,
        crate::api::http::routes::admin::latency_graph,
        // Admin - ACL
        crate::api::http::routes::admin::acl_list,
        crate::api::http::routes::admin::acl_users,
        crate::api::http::routes::admin::acl_whoami,
        crate::api::http::routes::admin::acl_cat,
        crate::api::http::routes::admin::acl_genpass,
        crate::api::http::routes::admin::acl_log,
        crate::api::http::routes::admin::acl_dryrun,
        crate::api::http::routes::admin::acl_setuser,
        crate::api::http::routes::admin::acl_deluser,
        crate::api::http::routes::admin::acl_load,
        crate::api::http::routes::admin::acl_save,
        // Admin - Command Introspection
        crate::api::http::routes::admin::command_list,
        crate::api::http::routes::admin::command_count,
        crate::api::http::routes::admin::command_docs,
        crate::api::http::routes::admin::command_info,
        crate::api::http::routes::admin::command_getkeys,
        // RedisTimeSeries endpoints
        crate::api::http::routes::timeseries::ts_create,
        crate::api::http::routes::timeseries::ts_alter,
        crate::api::http::routes::timeseries::ts_add,
        crate::api::http::routes::timeseries::ts_madd,
        crate::api::http::routes::timeseries::ts_get,
        crate::api::http::routes::timeseries::ts_range,
        crate::api::http::routes::timeseries::ts_rev_range,
        crate::api::http::routes::timeseries::ts_mget,
        crate::api::http::routes::timeseries::ts_mrange,
        crate::api::http::routes::timeseries::ts_mrev_range,
        crate::api::http::routes::timeseries::ts_incr_by,
        crate::api::http::routes::timeseries::ts_decr_by,
        crate::api::http::routes::timeseries::ts_del,
        crate::api::http::routes::timeseries::ts_query_index,
        crate::api::http::routes::timeseries::ts_info,
        crate::api::http::routes::timeseries::ts_create_rule,
        crate::api::http::routes::timeseries::ts_delete_rule,
        // Vector Sets endpoints
        crate::api::http::routes::vectors::vadd,
        crate::api::http::routes::vectors::vrem,
        crate::api::http::routes::vectors::vsim,
        crate::api::http::routes::vectors::vcard,
        crate::api::http::routes::vectors::vdim,
        crate::api::http::routes::vectors::vemb,
        crate::api::http::routes::vectors::vismember,
        crate::api::http::routes::vectors::vlinks,
        crate::api::http::routes::vectors::vrandmember,
        crate::api::http::routes::vectors::vrange,
        crate::api::http::routes::vectors::vinfo,
        crate::api::http::routes::vectors::vgetattr,
        crate::api::http::routes::vectors::vsetattr,
    ),
    components(
        schemas(
            // Common schemas
            ErrorResponse,
            ErrorDetail,
            KeyInfo,
            PaginationParams,
            TtlInfo,
            // Key schemas
            ScanParams,
            DeleteKeysRequest,
            DeleteKeysResponse,
            ExistsRequest,
            ExistsResponse,
            ExpireRequest,
            ExpireResponse,
            TtlResponse,
            PersistResponse,
            TypeResponse,
            RenameRequest,
            RenameResponse,
            CopyRequest,
            CopyResponse,
            ScanResponse,
            KeysParams,
            KeysResponse,
            TouchRequest,
            TouchResponse,
            RandomKeyResponse,
            DumpResponse,
            RestoreRequest,
            RestoreResponse,
            ObjectInfoResponse,
            KeyInfoResponse,
            SortOrderSchema,
            SortRequest,
            SortStoreRequest,
            SortResponse,
            SortStoreResponse,
            // String schemas
            StringValue,
            SetStringRequest,
            SetStringResponse,
            GetDelResponse,
            MGetRequest,
            MGetResponse,
            MSetRequest,
            MSetResponse,
            IncrementRequest,
            IncrementResponse,
            AppendRequest,
            AppendResponse,
            StrLenResponse,
            GetRangeParams,
            GetRangeResponse,
            SetRangeRequest,
            SetRangeResponse,
            GetExParams,
            LcsRequest,
            LcsResponse,
            LcsMatchSchema,
            // Hash schemas
            SetHashRequest,
            SetHashNxRequest,
            GetMultipleFieldsRequest,
            HashIncrRequest,
            HashIncrFloatRequest,
            ScanHashQuery,
            RandomFieldQuery,
            HashFieldEntry,
            HashScanResponse,
            HashRandomFieldResponse,
            // Hash field expiration schemas
            ExpireConditionSchema,
            HExpireRequest,
            HPExpireRequest,
            HExpireAtRequest,
            HPExpireAtRequest,
            HFieldsRequest,
            HExpireFieldResult,
            HExpireResponse,
            // Redis 8.0+ hash schemas
            HGetExExpirationSchema,
            HGetExRequest,
            HGetExResponse,
            HSetExExpirationSchema,
            HSetExConditionSchema,
            HSetExRequest,
            HSetExResponse,
            HGetDelRequest,
            HGetDelResponse,
            // List schemas
            ListPushRequest,
            ListPushResponse,
            ListPopRequest,
            ListPopResponse,
            ListRangeQuery,
            ListLengthResponse,
            ListIndexQuery,
            ListIndexResponse,
            ListSetRequest,
            ListInsertRequest,
            ListInsertResponse,
            InsertPositionParam,
            ListRemoveRequest,
            ListRemoveResponse,
            ListTrimRequest,
            ListPosQuery,
            ListPosResponse,
            ListDirectionParam,
            ListMoveRequest,
            ListMoveResponse,
            BlockingPopRequest,
            BlockingPopResponse,
            BlockingMoveRequest,
            LMPopRequest,
            BLMPopRequest,
            LMPopResponse,
            BlockingPopStreamQuery,
            BLMPopStreamQuery,
            // Set schemas
            SetAddRequest,
            SetAddResponse,
            SetRemoveRequest,
            SetRemoveResponse,
            SetMembersResponse,
            SetIsMemberRequest,
            SetIsMemberResponse,
            SetMIsMemberRequest,
            SetMIsMemberResponse,
            SetCardResponse,
            SetRandMemberQuery,
            SetRandMemberResponse,
            SetPopRequest,
            SetPopResponse,
            SetMoveRequest,
            SetMoveResponse,
            SetAlgebraRequest,
            SetAlgebraResponse,
            SetAlgebraStoreRequest,
            SetAlgebraStoreResponse,
            SetInterCardRequest,
            SetInterCardResponse,
            SetScanQuery,
            SetScanResponse,
            // Sorted Set schemas
            ScoredMemberDto,
            ZAddOptionsDto,
            ZAddRequest,
            ZAddResponse,
            ZAddIncrRequest,
            ZAddIncrResponse,
            ZRemRequest,
            ZRemResponse,
            ZMScoreRequest,
            ZMScoreResponse,
            ZScoreResponse,
            ZIncrByRequest,
            ZIncrByResponse,
            ZCardResponse,
            ScoreRangeDto,
            LexRangeDto,
            ZCountRequest,
            ZCountResponse,
            ZLexCountRequest,
            ZRankResponse,
            ZRangeQuery,
            ZRangeResponse,
            ZRangeByScoreRequest,
            ZRangeByLexRequest,
            ZRangeByLexResponse,
            ZRangeStoreRequest,
            ZRangeStoreResponse,
            ZRemRangeByRankRequest,
            ZRemRangeByScoreRequest,
            ZRemRangeByLexRequest,
            ZRemRangeResponse,
            ZPopQuery,
            ZPopResponse,
            ZBPopRequest,
            ZBPopResponse,
            ZMPopRequest,
            ZBMPopRequest,
            ZMPopResponse,
            ZRandMemberQuery,
            ZRandMemberResponse,
            ZAggregateDto,
            ZSetAlgebraOptionsDto,
            ZSetAlgebraRequest,
            ZSetAlgebraResponse,
            ZSetAlgebraStoreRequest,
            ZSetAlgebraStoreResponse,
            ZInterCardRequest,
            ZInterCardResponse,
            ZDiffRequest,
            ZDiffStoreRequest,
            ZScanQuery,
            ZScanResponse,
            ZBPopStreamQuery,
            // Bitmap schemas (core Redis)
            BitSetRequest,
            BitSetResponse,
            BitGetResponse,
            BitCountQuery,
            BitCountResponse,
            BitPosQuery,
            BitPosResponse,
            BitOpType,
            BitOpRequest,
            BitOpResponse,
            BitfieldEncodingSchema,
            BitfieldOverflowSchema,
            BitfieldCommandSchema,
            BitfieldRequest,
            BitfieldResponse,
            // Stream schemas
            StreamEntry,
            StreamInfo,
            ConsumerGroupInfo,
            ConsumerInfo,
            PendingEntry,
            PendingSummary,
            StreamReadResult,
            ClaimResult,
            AutoClaimResult,
            StreamAddRequest,
            StreamAddResponse,
            StreamRangeQuery,
            StreamEntriesResponse,
            StreamLengthResponse,
            StreamDeleteRequest,
            StreamDeleteResponse,
            TrimStrategyParam,
            StreamTrimRequest,
            StreamTrimResponse,
            StreamInfoQuery,
            StreamIdPair,
            StreamReadRequest,
            StreamReadBlockingRequest,
            StreamSubscribeQuery,
            StreamGroupSubscribeQuery,
            StreamReadGroupRequest,
            StreamReadGroupBlockingRequest,
            StreamAckRequest,
            StreamAckResponse,
            PendingQuery,
            StreamClaimRequest,
            StreamAutoClaimRequest,
            ConsumerGroupCreateRequest,
            ConsumerGroupCreateResponse,
            ConsumerGroupSetIdRequest,
            ConsumerCreateRequest,
            ConsumerOperationResponse,
            StreamSetIdRequest,
            // Admin - Pool & Capabilities
            PoolStats,
            RedisCapabilities,
            ModuleCapabilities,
            FeatureCapabilities,
            // Admin - Server info (domain entities)
            ServerInfo,
            ServerTime,
            DbSizeResponse,
            LastSaveResponse,
            // Admin - Memory (domain entities + request schemas)
            MemoryStats,
            MemoryUsageRequest,
            MemoryUsage,
            MemoryDoctorResponse,
            MemoryPurgeResponse,
            // Admin - Database operations (domain entity + request/response schemas)
            FlushDbRequest,
            FlushResult,
            CopyKeyRequest,
            CopyKeyResponse,
            MoveKeyRequest,
            MoveKeyResponse,
            SwapDbRequest,
            SwapDbResponse,
            // Admin - Config
            ConfigGetRequest,
            ConfigGetResponse,
            ConfigSetRequest,
            ConfigSetResponse,
            ConfigRewriteResponse,
            ConfigResetStatResponse,
            // Admin - Persistence (domain entities + response schema)
            SaveResponse,
            BgSaveResult,
            BgRewriteAofResult,
            // Admin - Client (domain entity for ClientInfo)
            ClientInfo,
            ClientListResponse,
            ClientKillRequest,
            ClientKillResponse,
            ClientPauseRequest,
            ClientPauseResponse,
            ClientUnpauseResponse,
            ClientSetNameRequest,
            ClientSetNameResponse,
            ClientGetNameResponse,
            ClientIdResponse,
            // Admin - Slowlog
            SlowlogEntry,
            SlowlogGetRequest,
            SlowlogGetResponse,
            SlowlogLenResponse,
            SlowlogResetResponse,
            // Admin - Latency
            LatencyEvent,
            LatencyLatestResponse,
            LatencyHistoryRequest,
            LatencyHistoryResponse,
            LatencyDoctorResponse,
            LatencyResetRequest,
            LatencyResetResponse,
            LatencyGraphResponse,
            // Admin - ACL
            AclListResponse,
            AclUsersResponse,
            AclWhoamiResponse,
            AclCatRequest,
            AclCatResponse,
            AclGenPassRequest,
            AclGenPassResponse,
            AclLogRequest,
            AclLogEntry,
            AclLogResponse,
            AclDryrunRequest,
            AclDryrunResponse,
            AclSetUserRequest,
            AclSetUserResponse,
            AclDelUserRequest,
            AclDelUserResponse,
            AclLoadResponse,
            AclSaveResponse,
            // Admin - Server
            DebugObjectRequest,
            DebugObjectResponse,
            ShutdownRequest,
            ShutdownResponse,
            // Admin - Client
            ClientInfoResponse,
            // Admin - Command Introspection
            CommandListQuery,
            CommandListResponse,
            CommandCountResponse,
            CommandDocsRequest,
            CommandInfoRequest,
            CommandGetKeysRequest,
            CommandGetKeysResponse,
            // JSON schemas (RedisJSON module)
            JsonSetRequest,
            JsonSetResponse,
            JsonGetParams,
            JsonGetResponse,
            JsonDelParams,
            JsonDelResponse,
            JsonMGetRequest,
            JsonMGetItem,
            JsonMGetResponse,
            JsonMSetItemRequest,
            JsonMSetRequest,
            JsonTypeParams,
            JsonTypeResponse,
            JsonStrLenParams,
            JsonStrLenResponse,
            JsonStrAppendRequest,
            JsonStrAppendResponse,
            JsonNumIncrByRequest,
            JsonNumMultByRequest,
            JsonNumResponse,
            JsonToggleParams,
            JsonToggleResponse,
            JsonClearParams,
            JsonClearResponse,
            JsonArrLenParams,
            JsonArrLenResponse,
            JsonArrAppendRequest,
            JsonArrAppendResponse,
            JsonArrIndexRequest,
            JsonArrIndexResponse,
            JsonArrInsertRequest,
            JsonArrInsertResponse,
            JsonArrPopRequest,
            JsonArrPopResponse,
            JsonArrTrimRequest,
            JsonArrTrimResponse,
            JsonObjLenParams,
            JsonObjLenResponse,
            JsonObjKeysParams,
            JsonObjKeysResponse,
            JsonDebugMemoryParams,
            JsonDebugMemoryResponse,
            JsonRespParams,
            JsonRespResponse,
            // Search schemas (RediSearch module)
            // Index operations
            CreateIndexRequest,
            CreateIndexResponse,
            IndexCreateOptionsDto,
            SearchFieldSchemaDto,
            DropIndexParams,
            DropIndexResponse,
            ListIndicesResponse,
            IndexInfoResponse,
            AlterIndexRequest,
            AlterIndexResponse,
            // Search/Aggregate operations
            SearchRequest,
            SearchOptionsDto,
            SearchResponse,
            AggregateRequest,
            AggregateOptionsDto,
            AggregateResponse,
            HybridSearchRequest,
            HybridSearchResponse,
            ExplainRequest,
            ExplainResponse,
            ProfileRequest,
            ProfileResponse,
            // Alias operations
            AliasRequest,
            AliasResponse,
            // Autocomplete operations
            SugAddRequest,
            SugAddResponse,
            SugGetParams,
            SugGetResponse,
            SugDelRequest,
            SugDelResponse,
            SugLenResponse,
            // Synonym operations
            SynonymDumpResponse,
            SynonymUpdateRequest,
            SynonymUpdateResponse,
            // Spellcheck operations
            SpellcheckRequest,
            SpellcheckResponse,
            // Dictionary operations
            DictTermsRequest,
            DictResponse,
            DictDumpResponse,
            // Search config
            SearchConfigGetResponse,
            SearchConfigSetRequest,
            SearchConfigSetResponse,
            // Search cursors
            CursorReadParams,
            CursorReadResponse,
            CursorDelResponse,
            // Search domain entities
            SearchFieldType,
            SearchFieldSchema,
            IndexDataType,
            IndexCreateOptions,
            IndexInfo,
            SearchOptions,
            SearchDocument,
            SearchResult,
            GeoFilter,
            NumericFilter,
            SortBy,
            SortOrder,
            HighlightOptions,
            SummarizeOptions,
            AggregateOptions,
            AggregateStep,
            AggregateResult,
            GroupByStep,
            SortByStep,
            ApplyStep,
            LimitStep,
            FilterStep,
            Reducer,
            SugAddOptions,
            SugGetOptions,
            Suggestion,
            SynonymGroup,
            SpellcheckResult,
            SpellcheckTerm,
            SpellcheckSuggestion,
            ProfileType,
            PhoneticMatcher,
            VectorAlgorithm,
            DistanceMetric,
            VectorFieldOptions,
            // Bloom filter schemas (RedisBloom module)
            BloomReserveRequest,
            BloomReserveResponse,
            BloomAddRequest,
            BloomAddResponse,
            BloomExistsRequest,
            BloomExistsResponse,
            BloomInsertRequest,
            BloomInsertResponse,
            BloomInfoResponse,
            BloomCardResponse,
            BloomScanDumpParams,
            BloomScanDumpResponse,
            BloomLoadChunkRequest,
            BloomLoadChunkResponse,
            // Cuckoo filter schemas (RedisBloom module)
            CuckooReserveRequest,
            CuckooReserveResponse,
            CuckooAddRequest,
            CuckooAddResponse,
            CuckooExistsRequest,
            CuckooExistsResponse,
            CuckooInsertRequest,
            CuckooInsertResponse,
            CuckooDelRequest,
            CuckooDelResponse,
            CuckooCountRequest,
            CuckooCountResponse,
            CuckooInfoResponse,
            CuckooScanDumpParams,
            CuckooScanDumpResponse,
            CuckooLoadChunkRequest,
            CuckooLoadChunkResponse,
            // Count-Min Sketch schemas (RedisBloom module)
            CmsInitByDimRequest,
            CmsInitByProbRequest,
            CmsInitResponse,
            CmsIncrByItem,
            CmsIncrByRequest,
            CmsIncrByResponse,
            CmsQueryRequest,
            CmsQueryResponse,
            CmsMergeRequest,
            CmsMergeResponse,
            CmsInfoResponse,
            // Top-K schemas (RedisBloom module)
            TopKReserveRequest,
            TopKReserveResponse,
            TopKAddRequest,
            TopKAddResponse,
            TopKIncrByItem,
            TopKIncrByRequest,
            TopKIncrByResponse,
            TopKQueryRequest,
            TopKQueryResponse,
            TopKCountResponse,
            TopKListQuery,
            TopKListItem,
            TopKListResponse,
            TopKInfoResponse,
            // T-Digest schemas (RedisBloom module)
            TDigestCreateRequest,
            TDigestAddRequest,
            TDigestQuantileRequest,
            TDigestValuesRequest,
            TDigestRanksRequest,
            TDigestMergeRequest,
            TDigestTrimmedMeanRequest,
            TDigestAckResponse,
            TDigestValuesResponse,
            TDigestRanksResponse,
            TDigestScalarResponse,
            TDigestInfoResponse,
            // HyperLogLog schemas (core Redis)
            PfAddRequest,
            PfAddResponse,
            PfCountRequest,
            PfCountResponse,
            PfMergeRequest,
            PfMergeResponse,
            // Geo schemas (core Redis)
            GeoUnitSchema,
            GeoSortOrderSchema,
            GeoPositionSchema,
            GeoMemberSchema,
            GeoAddRequest,
            GeoAddResponse,
            GeoPosRequest,
            GeoPosResponse,
            GeoDistQuery,
            GeoDistResponse,
            GeoHashRequest,
            GeoHashResponse,
            GeoSearchCenterSchema,
            GeoSearchShapeSchema,
            GeoSearchOptionsSchema,
            GeoSearchRequest,
            GeoSearchResponse,
            GeoSearchResultItem,
            GeoSearchStoreRequest,
            GeoSearchStoreResponse,
            GeoRadiusQuery,
            GeoRadiusByMemberQuery,
            // Pub/Sub schemas (core Redis)
            PublishRequest,
            PublishResponse,
            ChannelsResponse,
            NumSubRequest,
            NumSubItem,
            NumSubResponse,
            NumPatResponse,
            PubSubStatsResponse,
            PubSubMessage,
            WebSocketError,
            SubscriptionConfirmation,
            // Transaction schemas
            TransactionRequest,
            TransactionResponse,
            RedisCommand,
            CommandResult,
            CompareAndSetRequest,
            CompareAndSetResponse,
            HCompareAndSetRequest,
            KeyValue,
            FieldValue,
            TransactionScoredMember,
            // Scripting schemas (core Redis)
            EvalRequest,
            EvalShaRequest,
            EvalResponse,
            ScriptLoadRequest,
            ScriptLoadResponse,
            ScriptExistsRequest,
            ScriptExistsResult,
            ScriptExistsResponse,
            ScriptFlushRequest,
            FlushMode,
            ScriptFlushResponse,
            ScriptKillResponse,
            ScriptDebugMode,
            ScriptDebugRequest,
            ScriptDebugResponse,
            // Redis Functions schemas
            FunctionLoadRequest,
            FunctionLoadResponse,
            FunctionFlushModeSchema,
            FunctionFlushRequest,
            FunctionSuccessResponse,
            FunctionListQuery,
            FunctionListResponse,
            FunctionCallRequest,
            FunctionCallResponse,
            FunctionDumpResponse,
            FunctionRestorePolicySchema,
            FunctionRestoreRequest,
            FunctionStatsResponse,
            // RedisTimeSeries schemas
            Sample,
            DuplicatePolicy,
            Aggregation,
            TimeSeriesCreateRequest,
            TimeSeriesAddRequest,
            TimeSeriesRangeQuery,
            TimeSeriesMGetRequest,
            TimeSeriesMRangeRequest,
            TimeSeriesWriteResponse,
            TimeSeriesGetResponse,
            TimeSeriesRangeResponse,
            TimeSeriesMGetItem,
            TimeSeriesMGetResponse,
            TimeSeriesRangeItem,
            TimeSeriesMRangeResponse,
            TsAlterRequest,
            TsMaddItem,
            TsMaddRequest,
            TsMaddResponse,
            TsIncrDecrRequest,
            TsDelQuery,
            TsDelResponse,
            TsMrevRangeRequest,
            TsQueryIndexRequest,
            TsQueryIndexResponse,
            TsInfoResponse,
            TsCreateRuleRequest,
            // Vector Sets schemas
            VectorAddRequest,
            VectorAddResponse,
            VectorRemRequest,
            VectorRemResponse,
            VectorSimRequest,
            VectorSimItemResponse,
            VectorSimResponse,
            VectorCardResponse,
            VectorDimResponse,
            VectorEmbRequest,
            VectorEmbResponse,
            VectorIsMemberRequest,
            VectorIsMemberResponse,
            VectorLinksLayer,
            VectorLinksResponse,
            VectorRandMemberRequest,
            VectorRandMemberResponse,
            VectorRangeRequest,
            VectorRangeItemResponse,
            VectorRangeResponse,
            VectorInfoResponse,
            VectorGetAttrResponse,
            VectorSetAttrRequest,
            VectorSetAttrResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security scheme modifier for OpenAPI
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Admin-Api-Key"))),
            );
        }
    }
}

/// Path prefixes that belong to each capability-gated feature/module.
/// When a capability is disabled at runtime, all matching paths are stripped
/// from the served OpenAPI spec so the documentation stays in sync with the
/// live router (which returns 501 for unavailable features).
const STREAM_PREFIXES: &[&str] = &["/api/v1/streams"];
const JSON_PREFIXES: &[&str] = &["/api/v1/json"];
const SEARCH_PREFIXES: &[&str] = &["/api/v1/search"];
const BLOOM_PREFIXES: &[&str] = &[
    "/api/v1/bloom",
    "/api/v1/cms",
    "/api/v1/topk",
    "/api/v1/tdigest",
];
const TIMESERIES_PREFIXES: &[&str] = &["/api/v1/timeseries"];
const FUNCTIONS_PREFIXES: &[&str] = &["/api/v1/functions"];
const CLUSTER_PREFIXES: &[&str] = &["/api/v1/cluster"];
const VECTORS_PREFIXES: &[&str] = &["/api/v1/vectors"];

/// Tags removed together with their paths so the Swagger sidebar stays clean.
const STREAM_TAGS: &[&str] = &["Streams", "Streams (Admin)"];
const JSON_TAGS: &[&str] = &["JSON"];
const SEARCH_TAGS: &[&str] = &["Search"];
const BLOOM_TAGS: &[&str] = &[
    "Bloom Filters",
    "Cuckoo Filters",
    "Count-Min Sketch",
    "Top-K",
    "T-Digest",
];
const TIMESERIES_TAGS: &[&str] = &["TimeSeries"];
const FUNCTIONS_TAGS: &[&str] = &["Functions"];
const CLUSTER_TAGS: &[&str] = &["Cluster"];
const VECTORS_TAGS: &[&str] = &["Vectors"];

/// Build an OpenAPI spec filtered to only include routes that are actually
/// available given the detected Redis capabilities.
pub fn filtered_openapi(
    capabilities: &crate::infrastructure::redis::capabilities::RedisCapabilities,
) -> utoipa::openapi::OpenApi {
    let mut spec = ApiDoc::openapi();

    // Collect prefixes and tags to remove for disabled capabilities
    let mut remove_prefixes: Vec<&str> = Vec::new();
    let mut remove_tags: Vec<&str> = Vec::new();

    if !capabilities.features.streams {
        remove_prefixes.extend(STREAM_PREFIXES);
        remove_tags.extend(STREAM_TAGS);
    }
    if !capabilities.modules.json {
        remove_prefixes.extend(JSON_PREFIXES);
        remove_tags.extend(JSON_TAGS);
    }
    if !capabilities.modules.search {
        remove_prefixes.extend(SEARCH_PREFIXES);
        remove_tags.extend(SEARCH_TAGS);
    }
    if !capabilities.modules.bloom {
        remove_prefixes.extend(BLOOM_PREFIXES);
        remove_tags.extend(BLOOM_TAGS);
    }
    if !capabilities.modules.timeseries {
        remove_prefixes.extend(TIMESERIES_PREFIXES);
        remove_tags.extend(TIMESERIES_TAGS);
    }
    if !capabilities.features.functions {
        remove_prefixes.extend(FUNCTIONS_PREFIXES);
        remove_tags.extend(FUNCTIONS_TAGS);
    }
    if !capabilities.features.cluster {
        remove_prefixes.extend(CLUSTER_PREFIXES);
        remove_tags.extend(CLUSTER_TAGS);
    }
    if !capabilities.features.vectors {
        remove_prefixes.extend(VECTORS_PREFIXES);
        remove_tags.extend(VECTORS_TAGS);
    }

    // VRANGE is gated independently — strip it if vector_range is false
    // even when core vector commands are available.
    if !capabilities.features.vector_range {
        spec.paths.paths.remove("/api/v1/vectors/{key}/range");
    }

    if !remove_prefixes.is_empty() {
        // Filter paths
        spec.paths.paths.retain(|path, _| {
            !remove_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix))
        });

        // Filter tags
        if let Some(ref mut tags) = spec.tags {
            tags.retain(|tag| !remove_tags.contains(&tag.name.as_str()));
        }
    }

    spec
}

/// Create OpenAPI routes with Swagger UI.
///
/// Accepts capabilities so the served spec only advertises routes that are
/// actually mounted on the live router.
pub fn openapi_routes(
    capabilities: &crate::infrastructure::redis::capabilities::RedisCapabilities,
) -> Router<AppState> {
    let spec = filtered_openapi(capabilities);
    Router::new().merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::capabilities::RedisCapabilities;

    #[test]
    fn test_openapi_security_scheme() {
        let spec = ApiDoc::openapi();
        let components = spec.components.expect("components");
        assert!(components.security_schemes.contains_key("api_key"));
    }

    #[test]
    fn test_filtered_openapi_removes_json_paths_when_module_disabled() {
        let mut caps = RedisCapabilities::default_capabilities();
        caps.modules.json = false;
        let spec = filtered_openapi(&caps);
        let has_json_path = spec
            .paths
            .paths
            .keys()
            .any(|p| p.starts_with("/api/v1/json"));
        assert!(
            !has_json_path,
            "JSON paths should be removed when module is disabled"
        );
    }

    #[test]
    fn test_filtered_openapi_keeps_json_paths_when_module_enabled() {
        let mut caps = RedisCapabilities::default_capabilities();
        caps.modules.json = true;
        let spec = filtered_openapi(&caps);
        let has_json_path = spec
            .paths
            .paths
            .keys()
            .any(|p| p.starts_with("/api/v1/json"));
        assert!(
            has_json_path,
            "JSON paths should be present when module is enabled"
        );
    }

    #[test]
    fn test_filtered_openapi_removes_bloom_paths_when_module_disabled() {
        let mut caps = RedisCapabilities::default_capabilities();
        caps.modules.bloom = false;
        let spec = filtered_openapi(&caps);
        assert!(
            !spec
                .paths
                .paths
                .keys()
                .any(|p| p.starts_with("/api/v1/bloom"))
        );
        assert!(
            !spec
                .paths
                .paths
                .keys()
                .any(|p| p.starts_with("/api/v1/cms"))
        );
        assert!(
            !spec
                .paths
                .paths
                .keys()
                .any(|p| p.starts_with("/api/v1/topk"))
        );
        assert!(
            !spec
                .paths
                .paths
                .keys()
                .any(|p| p.starts_with("/api/v1/tdigest"))
        );
    }

    #[test]
    fn test_filtered_openapi_all_enabled_retains_all_paths() {
        let mut caps = RedisCapabilities::default_capabilities();
        caps.modules.json = true;
        caps.modules.search = true;
        caps.modules.bloom = true;
        caps.modules.timeseries = true;
        caps.features.streams = true;
        caps.features.functions = true;
        caps.features.cluster = true;
        caps.features.vectors = true;
        caps.features.vector_range = true;
        let full_spec = ApiDoc::openapi();
        let filtered = filtered_openapi(&caps);
        assert_eq!(full_spec.paths.paths.len(), filtered.paths.paths.len());
    }
}
