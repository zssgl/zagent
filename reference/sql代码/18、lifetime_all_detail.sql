

-- 步骤 0: 初始化会话变量
-- ---------------------------------------------------------------------
-- (保持不变) 定义诊所变量
SET @clinic = CONVERT('杭州颜术悦容医疗美容诊所,杭州颜术时尚医疗美容诊所,杭州颜术新芽医疗美容诊所,苏州颜术医疗美容诊所有限公司,杭州颜术素颜丰潭医疗美容诊所,上海颜术高得医疗美容门诊部' USING utf8mb4) COLLATE utf8mb4_general_ci;

-- 初始化用于模拟窗口函数的变量
SET @unique_row_id := 0; -- 用于 ROW_NUMBER()
SET @partition_col := NULL, @order_col := NULL, @dense_rank := 0; -- 用于 DENSE_RANK()


-- 步骤 1: 创建【数据准备】临时表 (替代 DateGrouped CTE)
-- ---------------------------------------------------------------------
-- 功能：筛选所有相关原始消费记录，并使用变量模拟 ROW_NUMBER() 添加唯一行ID。
DROP TEMPORARY TABLE IF EXISTS DateGrouped_temp;
CREATE TEMPORARY TABLE DateGrouped_temp (
    -- 添加索引以优化后续的 JOIN 和 WHERE 操作
    PRIMARY KEY (unique_row_id),
    INDEX(病历编号, 时间段),
    INDEX(病历编号, 消耗日期)
) ENGINE=MEMORY AS
SELECT
    rcd.病历编号,
    rcd.项目商品,
    rcd.项目ID, 
    rcd.实质消费,
    DATE(rcd.消耗时间) AS 消耗日期,
    DATE_FORMAT(rcd.消耗时间, '%Y-%m') AS 时间段,
    -- 使用用户变量模拟 ROW_NUMBER() OVER()
    @unique_row_id := @unique_row_id + 1 AS unique_row_id
FROM
    report_consumption_detail rcd
    LEFT JOIN customers cu ON rcd.病历编号 = cu.CID
    LEFT JOIN organizations org ON cu.OrganizationID = org.ID
    LEFT JOIN employees emp ON cu.CustomerServerID = emp.id
    LEFT JOIN employees emp2 ON cu.DoctorID = emp2.id
WHERE
    FIND_IN_SET(rcd.消耗诊所, @clinic)
    AND DATE(rcd.消耗时间) >= '2023-01-01'
    AND rcd.实质消费 > 0
    AND NOT ((rcd.顾客姓名 LIKE '%q%' OR rcd.顾客姓名 LIKE '%Q%') AND rcd.消耗诊所 = '杭州颜术悦容医疗美容诊所');


-- 步骤 2: 创建【消费排序】与【有效消费】临时表
-- ---------------------------------------------------------------------
-- 功能: 将原版的 RankedConsumption 和 FilteredConsumption 两个步骤合并，直接生成有效消费记录。
DROP TEMPORARY TABLE IF EXISTS FilteredConsumption_temp;
CREATE TEMPORARY TABLE FilteredConsumption_temp (
    INDEX(病历编号, 消耗日期)
) ENGINE=MEMORY AS
SELECT
    t.病历编号,
    t.消耗日期
FROM (
    SELECT
        dg.病历编号,
        dg.消耗日期,
        dg.实质消费,
        -- 核心：模拟 DENSE_RANK()
        @dense_rank := IF(@partition_col = dg.病历编号,
                          IF(@order_col = dg.消耗日期, @dense_rank, @dense_rank + 1),
                          1) AS 消费序号,
        @partition_col := dg.病历编号,
        @order_col := dg.消耗日期
    FROM
        -- 必须预先排序以保证变量计算正确
        (SELECT * FROM DateGrouped_temp ORDER BY 病历编号, 消耗日期) AS dg
) AS t
WHERE
    (t.消费序号 = 1) OR (t.消费序号 >= 2 AND t.实质消费 > 300);


-- 步骤 3: 创建【按月统计】临时表 (替代 MonthlySummary CTE)
-- ---------------------------------------------------------------------
-- 功能: 按月统计每个病人的“有效消费天数”。
DROP TEMPORARY TABLE IF EXISTS MonthlySummary_temp;
CREATE TEMPORARY TABLE MonthlySummary_temp (
    INDEX(病历编号, TheMonth)
) ENGINE=MEMORY AS
SELECT
    病历编号,
    DATE_FORMAT(消耗日期, '%Y-%m') AS TheMonth,
    COUNT(DISTINCT 消耗日期) AS DistinctEffectiveDays
FROM
    FilteredConsumption_temp
GROUP BY
    病历编号, TheMonth;


-- 步骤 4: [新增修复步骤] 创建【未来状态辅助】临时表
-- ---------------------------------------------------------------------
-- 功能: 预先计算出哪些月份存在未来的有效消费月，以解决 "Can't reopen table" 错误。
DROP TEMPORARY TABLE IF EXISTS FutureStatus_temp;
CREATE TEMPORARY TABLE FutureStatus_temp (
    INDEX(病历编号, TheMonth)
) ENGINE=MEMORY AS
SELECT DISTINCT
    all_months.病历编号,
    all_months.时间段 AS TheMonth
FROM
    (SELECT DISTINCT 病历编号, 时间段 FROM DateGrouped_temp) AS all_months
JOIN MonthlySummary_temp ms_future
    ON all_months.病历编号 = ms_future.病历编号 AND all_months.时间段 < ms_future.TheMonth;


-- 步骤 5: 创建【月度复购状态】临时表 (替代 MonthlyStatus CTE, 已重构)
-- ---------------------------------------------------------------------
-- 功能: 判定每个病人每个消费月份的最终复购状态 ('Y' 或 'N')。
DROP TEMPORARY TABLE IF EXISTS MonthlyStatus_temp;
CREATE TEMPORARY TABLE MonthlyStatus_temp (
    INDEX(病历编号, TheMonth)
) ENGINE=MEMORY AS
SELECT
    all_months.病历编号,
    all_months.时间段 AS TheMonth,
    -- 使用 JOIN 替代 EXISTS 子查询来判断未来状态
    CASE
        WHEN ms.DistinctEffectiveDays >= 2 OR fs.TheMonth IS NOT NULL THEN 'Y'
        ELSE 'N'
    END AS FinalStatus
FROM
    (SELECT DISTINCT 病历编号, 时间段 FROM DateGrouped_temp) AS all_months
LEFT JOIN MonthlySummary_temp ms
    ON all_months.病历编号 = ms.病历编号 AND all_months.时间段 = ms.TheMonth
LEFT JOIN FutureStatus_temp fs -- 使用上一步创建的辅助表
    ON all_months.病历编号 = fs.病历编号 AND all_months.时间段 = fs.TheMonth;


-- 步骤 6: 创建【后续日期关联】临时表 (替代 SubsequentDateLinks CTE)
-- ---------------------------------------------------------------------
-- 功能: 为每一笔原始消费，找出所有发生在它之后的有效消费日期。
DROP TEMPORARY TABLE IF EXISTS SubsequentDateLinks_temp;
CREATE TEMPORARY TABLE SubsequentDateLinks_temp (
    INDEX(unique_row_id)
) ENGINE=MEMORY AS
SELECT
    dg.unique_row_id,
    fc.消耗日期 AS 后续具体日期
FROM
    DateGrouped_temp dg
JOIN
    FilteredConsumption_temp fc ON dg.病历编号 = fc.病历编号 AND fc.消耗日期 > dg.消耗日期;


-- 步骤 7: 最终整合查询
-- ---------------------------------------------------------------------
-- 功能: 将所有准备好的数据连接起来，计算复购周期并根据最终逻辑进行过滤和排序。


DROP TABLE IF EXISTS lifetime_all_detail;

-- 2️⃣ 创建新表
CREATE TABLE lifetime_all_detail (
    当前时间段 VARCHAR(10),
    病历编号 VARCHAR(100),
    最早开单日期 DATE,
    项目商品 VARCHAR(200),
    项目ID  VARCHAR(255),
    当前具体日期 DATE,
    后续时间段 VARCHAR(10),
    后续具体日期 DATE,
    第二次是否复购 CHAR(1),
    复购周期 VARCHAR(10),
    复购排序 INT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- 3️⃣ 插入数据
INSERT INTO lifetime_all_detail (
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
		DATE_FORMAT(sdl.后续具体日期, '%Y-%m') AS 后续时间段,
    sdl.后续具体日期,
    -- 直接使用我们第一步算好的月度状态
    ms.FinalStatus AS 第二次是否复购,
CASE
    -- 1. 优先：如果能找到具体的后续日期，正常计算周期
    WHEN sdl.后续具体日期 IS NOT NULL THEN
        CONCAT('N+', (YEAR(sdl.后续具体日期) * 12 + MONTH(sdl.后续具体日期)) - (YEAR(dg.消耗日期) * 12 + MONTH(dg.消耗日期)))
    -- 2. 新增：如果找不到后续日期，但该月状态是'Y'，则显示'N+0'
    WHEN ms.FinalStatus = 'Y' AND sdl.后续具体日期 IS NULL THEN
        'N+0'
    -- 3. 其他所有情况（即状态为'N'），显示'未回购'
    ELSE
        '未回购'
END AS 复购周期,
CASE
    -- 1. 如果有后续日期，计算月份差作为排序值
    WHEN sdl.后续具体日期 IS NOT NULL THEN
        (YEAR(sdl.后续具体日期) * 12 + MONTH(sdl.后续具体日期)) - (YEAR(dg.消耗日期) * 12 + MONTH(dg.消耗日期))
    -- 2. 如果没有后续日期，但状态是'Y'，排序值为0
    WHEN ms.FinalStatus = 'Y' AND sdl.后续具体日期 IS NULL THEN
        0
    -- 3. 其他情况（即不复购），给一个很大的值，让它排在最后
    ELSE
        9999
END AS 复购排序
FROM
    DateGrouped_temp dg
-- JOIN月度状态表，赋予每条记录'Y'或'N'的状态
JOIN MonthlyStatus_temp ms
    ON dg.病历编号 = ms.病历编号 AND dg.时间段 = ms.TheMonth
-- LEFT JOIN后续日期表，如果能找到后续日期，就列出来；找不到则为NULL
LEFT JOIN SubsequentDateLinks_temp sdl
    ON dg.unique_row_id = sdl.unique_row_id
-- 最后只输出状态为'Y'的记录，或者状态为'N'且没有后续日期的记录
LEFT JOIN(
 SELECT
    病历编号,
    DATE(MIN(最早开单时间))               AS 最早开单日期,
    DATE_FORMAT(MIN(最早开单时间), '%Y-%m') AS 最早开单时间段
  FROM report_bill_detail
  GROUP BY 病历编号

) bf
ON dg.病历编号 =bf.病历编号
WHERE
    ms.FinalStatus = 'Y' OR sdl.后续具体日期 IS NULL
ORDER BY
    dg.病历编号, dg.消耗日期, sdl.后续具体日期;
