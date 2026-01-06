-- Step 1: Create empty table first
DROP TABLE IF EXISTS report_consumption_nurse;
CREATE TABLE report_consumption_nurse (
 病历编号 VARCHAR(255),
    顾客姓名 VARCHAR(255),
    消耗时间 DATETIME,
    `项目ID` VARCHAR(255),
    项目商品 VARCHAR(255),
    消耗诊所 VARCHAR(255),
    消耗数量 INT,
     `订单来源` INT,
     
    `是否是内购订单` INT,
   护士咨询业绩 INT,
   nurse_count INT,
   nurse_name VARCHAR(255)
);

-- Step 2: Insert data
INSERT INTO report_consumption_nurse
SELECT * FROM (
   SELECT 病历编号, 顾客姓名,消耗时间,`项目ID`,项目商品,消耗诊所, 消耗数量,`订单来源`, `是否是内购订单`,护士咨询业绩, nurse_count,
       SUBSTRING_INDEX(nurses, ',', 1) as nurse_name
   FROM report_consumption_employee
   WHERE nurse_count IN (1,2,3)
   UNION ALL 
   SELECT 病历编号, 顾客姓名,消耗时间,`项目ID`,项目商品,消耗诊所, 消耗数量,`订单来源`,`是否是内购订单`, 护士咨询业绩, nurse_count,
       SUBSTRING_INDEX(SUBSTRING_INDEX(nurses, ',', 2), ',', -1) as nurse_name
   FROM report_consumption_employee
   WHERE nurse_count IN (2,3)
   UNION ALL
   SELECT 病历编号, 顾客姓名,消耗时间,`项目ID`,项目商品,消耗诊所, 消耗数量, `订单来源`,`是否是内购订单`,护士咨询业绩, nurse_count,
       SUBSTRING_INDEX(SUBSTRING_INDEX(nurses, ',', -1), ',', 1) as nurse_name
   FROM report_consumption_employee
   WHERE nurse_count = 3
) t 
WHERE nurse_name IS NOT NULL
ORDER BY 消耗时间 DESC;