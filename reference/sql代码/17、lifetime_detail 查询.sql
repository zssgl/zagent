



SET @clinic = CONVERT('杭州颜术悦容医疗美容诊所,杭州颜术时尚医疗美容诊所,杭州颜术新芽医疗美容诊所,苏州颜术医疗美容诊所有限公司,杭州颜术素颜丰潭医疗美容诊所,上海颜术高得医疗美容门诊部' USING utf8mb4) COLLATE utf8mb4_general_ci;


SET @partition_col := NULL, @order_col := NULL, @dense_rank := 0;



-- 功能：筛选出所有相关的原始消费记录。
DROP TEMPORARY TABLE IF EXISTS DateGrouped_temp;
CREATE TEMPORARY TABLE DateGrouped_temp (
    -- 为常用查询列添加索引以优化后续查询性能
    INDEX(病历编号),
    INDEX(消耗日期),
    INDEX(时间段)
) ENGINE=MEMORY AS -- 对于中小型结果集，使用内存(MEMORY)引擎可以加快处理速度
SELECT
    rcd.病历编号,
    rcd.项目商品,
    rcd.项目ID, 
    DATE(rcd.消耗时间) AS 消耗日期,
    rcd.实质消费,
    DATE_FORMAT(rcd.消耗时间, '%Y-%m') AS 时间段
FROM
    report_consumption_detail rcd
WHERE
    FIND_IN_SET(rcd.消耗诊所, @clinic)
    AND DATE(rcd.消耗时间) >= '2023-01-01'
    AND rcd.实质消费 > 0
    AND NOT ((rcd.顾客姓名 LIKE '%q%' OR rcd.顾客姓名 LIKE '%Q%') AND rcd.消耗诊所 = '杭州颜术悦容医疗美容诊所');


-- ---------------------------------------------------------------------
-- 功能：使用用户变量模拟 DENSE_RANK()，为每个病人的消费按日期进行排序。
DROP TEMPORARY TABLE IF EXISTS RankedConsumption_temp;
CREATE TEMPORARY TABLE RankedConsumption_temp (
    INDEX(病历编号),
    INDEX(消费序号)
) ENGINE=MEMORY AS
SELECT
    dg.病历编号,
    dg.项目商品,
    dg.消耗日期,
    dg.实质消费,
    dg.时间段,
    -- 核心逻辑：模拟 DENSE_RANK() OVER (PARTITION BY 病历编号 ORDER BY 消耗日期)
    @dense_rank := IF(@partition_col = dg.病历编号,
                      IF(@order_col = dg.消耗日期, @dense_rank, @dense_rank + 1),
                      1) AS 消费序号,
    @partition_col := dg.病历编号,
    @order_col := dg.消耗日期
FROM
    -- 必须预先对数据进行严格排序，这是保证变量计算正确的关键
    (SELECT * FROM DateGrouped_temp ORDER BY 病历编号, 消耗日期) AS dg;


-- 步骤 3: 创建【有效消费】临时表 (替代 FilteredConsumption CTE)
-- ---------------------------------------------------------------------
-- 功能：根据排序结果，筛选出定义为“有效消费”的记录。
DROP TEMPORARY TABLE IF EXISTS FilteredConsumption_temp;
CREATE TEMPORARY TABLE FilteredConsumption_temp (
    INDEX(病历编号),
    INDEX(消耗日期)
) ENGINE=MEMORY AS
SELECT
    病历编号,
    消耗日期
FROM
    RankedConsumption_temp
WHERE
    -- 条件：首次消费，或非首次消费但金额 > 300
    (消费序号 = 1) OR (消费序号 >= 2 AND 实质消费 > 300);


-- 步骤 4: 创建【按月统计】临时表 (替代 MonthlySummary CTE)
-- ---------------------------------------------------------------------
-- 功能：按月统计每个病人的“有效消费天数”和具体的日期列表。
DROP TEMPORARY TABLE IF EXISTS MonthlySummary_temp;
CREATE TEMPORARY TABLE MonthlySummary_temp (
    INDEX(病历编号, TheMonth)
) ENGINE=MEMORY AS
SELECT
    病历编号,
    DATE_FORMAT(消耗日期, '%Y-%m') AS TheMonth,
    COUNT(DISTINCT 消耗日期) AS DistinctEffectiveDays,
    GROUP_CONCAT(DISTINCT 消耗日期 ORDER BY 消耗日期 SEPARATOR ',') AS EffectiveDates
FROM
    FilteredConsumption_temp
GROUP BY
    病历编号, TheMonth;


-- 步骤 5: 创建【跨月复购辅助】临时表 (新增步骤以修复错误)
-- ---------------------------------------------------------------------
-- 功能: 预先计算跨月复购信息，以避免在后续步骤中出现 "Can't reopen table: '...'" 错误。
DROP TEMPORARY TABLE IF EXISTS CrossMonthRepurchase_temp;
CREATE TEMPORARY TABLE CrossMonthRepurchase_temp (
    INDEX(病历编号, TheMonth)
) ENGINE=MEMORY AS
SELECT
    am.病历编号,
    am.时间段 AS TheMonth,
    MIN(s.TheMonth) AS NextPurchaseMonth,
    -- 由于MySQL 5.7不支持更高级的窗口函数来直接获取与MIN(TheMonth)对应的日期，
    -- 但因为日期和月份是同步增长的，所以MIN(日期)必然属于MIN(月份)。
    MIN(CAST(SUBSTRING_INDEX(s.EffectiveDates, ',', 1) AS DATE)) AS NextPurchaseDate
FROM
    -- 从所有消费记录中提取出所有出现过的 病人-月份 组合
    (SELECT DISTINCT 病历编号, 时间段 FROM DateGrouped_temp) AS am
-- 将其与未来的有效消费月连接起来
JOIN
    MonthlySummary_temp s ON am.病历编号 = s.病历编号 AND am.时间段 < s.TheMonth
GROUP BY
    am.病历编号, am.时间段;


-- 步骤 6: 创建【复购决策】临时表 (替代 RepurchaseDecision CTE, 已重构)
-- ---------------------------------------------------------------------
-- 功能：核心逻辑，为每个病人的每个消费月份，按优先级决定复购状态和复购日期。
DROP TEMPORARY TABLE IF EXISTS RepurchaseDecision_temp;
CREATE TEMPORARY TABLE RepurchaseDecision_temp (
    INDEX(病历编号, TheMonth)
) ENGINE=MEMORY AS
SELECT
    all_months.病历编号,
    all_months.时间段 AS TheMonth,
    -- 步骤1: 判断是否满足【优先】逻辑 (当月复购)
    CASE
        WHEN ms.DistinctEffectiveDays >= 2 THEN 'Y'
        -- 步骤2: 判断是否满足【次级】逻辑 (跨月复购), 直接使用上一步预计算的结果
        WHEN cmr.NextPurchaseMonth IS NOT NULL THEN 'Y'
        -- 步骤3: 都不满足，则为不复购
        ELSE 'N'
    END AS 是否复购,

    -- 对应地，决定后续具体日期
    CASE
        WHEN ms.DistinctEffectiveDays >= 2 THEN
            -- 取当月第二个有效日
            CAST(SUBSTRING_INDEX(SUBSTRING_INDEX(ms.EffectiveDates, ',', 2), ',', -1) AS DATE)
        WHEN cmr.NextPurchaseMonth IS NOT NULL THEN
            -- 直接从辅助表中获取未来第一个有效月的第一个有效日
            cmr.NextPurchaseDate
        ELSE NULL
    END AS 后续具体日期
FROM
    (SELECT DISTINCT 病历编号, 时间段 FROM DateGrouped_temp) AS all_months
LEFT JOIN MonthlySummary_temp ms
    ON all_months.病历编号 = ms.病历编号 AND all_months.时间段 = ms.TheMonth
-- 使用新创建的辅助表进行关联，取代原有的子查询
LEFT JOIN CrossMonthRepurchase_temp cmr
    ON all_months.病历编号 = cmr.病历编号 AND all_months.时间段 = cmr.TheMonth;


-- 步骤 7: 最终查询 (与原版逻辑一致)
-- ---------------------------------------------------------------------
-- 功能：将计算好的复购结果关联回原始消费记录，并计算复购周期。
DROP TABLE IF EXISTS lifetime_detail;

-- 2️⃣ 创建新表
CREATE TABLE lifetime_detail (
    当前时间段 VARCHAR(10),
    病历编号 VARCHAR(100),
    最早开单日期 DATE,
    项目商品 VARCHAR(200),
    项目ID   VARCHAR(255), 
    当前具体日期 DATE,
    后续时间段 VARCHAR(10),
    后续具体日期 DATE,
    第二次是否复购 CHAR(1),
    复购周期 VARCHAR(10),
    复购排序 INT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- 3️⃣ 插入数据
INSERT INTO lifetime_detail (
    当前时间段,
    病历编号,
    最早开单日期,
    项目商品,
    项目ID, 
    当前具体日期,
    后续时间段,
    后续具体日期,
    第二次是否复购,
    复购周期,
    复购排序
)

SELECT
    dg.时间段 AS 当前时间段,
    dg.病历编号,
		bf.最早开单日期,
    dg.项目商品,
    dg.项目ID,  
    dg.消耗日期 AS 当前具体日期,
    DATE_FORMAT(rd.后续具体日期, '%Y-%m') AS 后续时间段,
    rd.后续具体日期,
    
    rd.是否复购 AS 第二次是否复购,
    CASE
        WHEN rd.是否复购 = 'Y' THEN
            CONCAT('N+',
                (YEAR(rd.后续具体日期) * 12 + MONTH(rd.后续具体日期)) - (YEAR(dg.消耗日期) * 12 + MONTH(dg.消耗日期))
            )
        ELSE '未回购'
    END AS 复购周期,
CASE
    WHEN rd.是否复购 = 'Y' THEN
         (YEAR(rd.后续具体日期) * 12 + MONTH(rd.后续具体日期)) - (YEAR(dg.消耗日期) * 12 + MONTH(dg.消耗日期))
    ELSE 9999
END AS 复购排序
FROM
    DateGrouped_temp dg
LEFT JOIN RepurchaseDecision_temp rd
    ON dg.病历编号 = rd.病历编号 AND dg.时间段 = rd.TheMonth
LEFT JOIN(
 SELECT
    病历编号,
    DATE(MIN(最早开单时间))               AS 最早开单日期,
    DATE_FORMAT(MIN(最早开单时间), '%Y-%m') AS 最早开单时间段
  FROM report_bill_detail
  GROUP BY 病历编号

) bf
ON dg.病历编号 =bf.病历编号
ORDER BY
    dg.病历编号, dg.消耗日期;
