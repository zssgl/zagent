DROP TABLE IF EXISTS report_consumption_detail;

-- Step 2: Create the table structure
CREATE TABLE report_consumption_detail (
    `顾客姓名` VARCHAR(255),
    `病历编号` VARCHAR(255),
    `性别` VARCHAR(10),
    `消耗时间` DATETIME,
    `类型` VARCHAR(255),
    `业绩人` VARCHAR(255),
    `项目分类` VARCHAR(255),
    `项目ID` VARCHAR(255),
    `项目商品` VARCHAR(255),
    `消耗数量` INT,
    `消耗金额` DECIMAL(15, 2),
    `消耗诊所` VARCHAR(255),
    `创建人` VARCHAR(255),
    `开单诊所` VARCHAR(255),
    `实质消费` DECIMAL(15, 2),
    `消耗金额（非欠费均分）` DECIMAL(15, 2),
    `促销活动` VARCHAR(255),
    `组合项目` VARCHAR(255),
    `提点类型` VARCHAR(255),
    `最早开单时间` DATETIME,
    `订单来源` INT,
    `是否是内购订单` INT
);

-- Step 3: Insert data into the table
INSERT INTO report_consumption_detail (
    `顾客姓名`, `病历编号`, `性别`, `消耗时间`, `类型`, `业绩人`, 
    `项目分类`, `项目ID`,`项目商品`, `消耗数量`, `消耗金额`, `消耗诊所`, 
    `创建人`, `开单诊所`, `实质消费`, `消耗金额（非欠费均分）`, 
    `促销活动`, `组合项目`, `提点类型`,`最早开单时间`,`订单来源`,`是否是内购订单`
)

SELECT
*
FROM report_consumption_detail_temp
WHERE `业绩人` != '练习测试'

UNION ALL

SELECT
*
FROM report_consumption_detail_sh