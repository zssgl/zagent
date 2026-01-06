-- Step 1: Create empty table first
DROP TABLE IF EXISTS report_bill_consultant;
CREATE TABLE report_bill_consultant (
    开单门店 VARCHAR(255),
    病历编号 VARCHAR(255),
    顾客姓名 VARCHAR(255),
    `项目ID` VARCHAR(255),
    开单项目 VARCHAR(255),
    数量 INT,
    客户来源 VARCHAR(255),
    付款时间 DATETIME,
    最早开单时间 DATETIME,
    最后消耗时间 DATETIME,
     `订单来源` INT,
      `是否是内购订单` INT,
    开单时间 DATETIME,
    护士咨询业绩 INT,
    cons_count INT,
    cons_name VARCHAR(255)
);

-- Step 2: Insert data
INSERT INTO report_bill_consultant
SELECT * FROM (
    SELECT 开单门店, 病历编号, 顾客姓名,`项目ID`, 开单项目,数量, 客户来源,
        付款时间, 最早开单时间, 最后消耗时间,`订单来源`, `是否是内购订单`,开单时间, 护士咨询业绩, cons_count,
        SUBSTRING_INDEX(consultants, ',', 1) as cons_name
    FROM report_bill_employee
    WHERE cons_count IN (1,2)
    UNION ALL 
    SELECT 开单门店, 病历编号, 顾客姓名, `项目ID`,开单项目,数量, 客户来源,
        付款时间, 最早开单时间, 最后消耗时间,`订单来源`,`是否是内购订单`, 开单时间, 护士咨询业绩, cons_count,
        SUBSTRING_INDEX(SUBSTRING_INDEX(consultants, ',', 2), ',', -1) as cons_name
    FROM report_bill_employee
    WHERE cons_count = 2
) t 
WHERE cons_name IS NOT NULL
ORDER BY 付款时间 DESC;